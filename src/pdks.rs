//! Ar-Ge Portal PDKS client.
//!
//! `jf pdks login` opens a sign-in window; after a successful portal session the
//! cookies (including HttpOnly) are copied into Keychain. `jf pdks` then uses
//! that cookie over HTTP to fetch punch records and print the weekly report.

use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Timelike};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};
use wry::{WebContext, WebViewBuilder};

use crate::cli::{PdksArgs, PdksCommand};
use crate::config::{PDKS_DAILY_TARGET_HOURS, PDKS_WEEKLY_TARGET_HOURS};
use crate::time::compute::{format_hours, get_week_start_monday};

const BASE_URL: &str = "https://argeportal.hacettepeteknokent.com.tr";
const KEYCHAIN_SERVICE: &str = "jotmate-pdks";
const KEY_SESSION: &str = "session-cookie";
const LOGIN_PROBE: &str = r#"(() => {
  const hasId = value => {
    if (Array.isArray(value)) return value.some(hasId);
    if (!value || typeof value !== 'object') return false;
    if (['id', 'Id', 'value', 'Value'].some(key => /^\d+$/.test(String(value[key] ?? '')))) return true;
    return Object.values(value).some(hasId);
  };
  const ping = async () => {
    try {
      const userResponse = await fetch('/Home/GetUserInfoForClient', {
        method: 'POST',
        credentials: 'include',
        headers: {'X-Requested-With': 'XMLHttpRequest'}
      });
      const user = JSON.parse(await userResponse.text());
      if (!user) return;
      const periodsResponse = await fetch('/ProjeSabit/FirmaDonem', {
        method: 'POST', credentials: 'include',
        headers: {'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8', 'X-Requested-With': 'XMLHttpRequest'},
        body: new URLSearchParams({ Firma_Id: '21' })
      });
      if (hasId(JSON.parse(await periodsResponse.text()))) {
        window.ipc.postMessage(JSON.stringify({ loggedIn: true }));
      }
    } catch (_) {}
  };
  ping();
  setInterval(ping, 1500);
})()"#;
#[derive(Debug, Clone, PartialEq, Eq)]
struct Punch {
    at: NaiveDateTime,
    state: PunchState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PunchState {
    Entry,
    Exit,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ReportDay {
    pub date: NaiveDate,
    pub first: NaiveDateTime,
    pub last: NaiveDateTime,
    pub worked_secs: i64,
    /// The final recorded movement is an entry, so `last` is the current time.
    pub is_still_in_office: bool,
}

impl ReportDay {
    pub fn target_exit(&self) -> NaiveDateTime {
        self.first + chrono::Duration::seconds((PDKS_DAILY_TARGET_HOURS * 3600.0) as i64)
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub week: NaiveDate,
    pub target_hours: f64,
    pub worked_secs: i64,
    pub days: Vec<ReportDay>,
}

pub async fn run(args: PdksArgs) -> Result<()> {
    match args.command {
        Some(PdksCommand::Login | PdksCommand::LoginHelper) => return login(),
        None => {}
    }

    if !(0.0..=PDKS_WEEKLY_TARGET_HOURS).contains(&args.target_hours) {
        anyhow::bail!("--target-hours must be between 0 and {PDKS_WEEKLY_TARGET_HOURS}");
    }
    let report = fetch_report_after_login(args).await?;
    print_report(&report);
    Ok(())
}

pub async fn fetch_report(args: PdksArgs) -> Result<Report> {
    let week = get_week_start_monday(args.week.unwrap_or_else(|| Local::now().date_naive()));
    let responses = fetch_with_saved_session(&args, week).await?;
    let punches: Vec<Punch> = responses.iter().flat_map(parse_punches).collect();
    Ok(build_report(week, args.target_hours, &punches))
}

async fn fetch_report_after_login(args: PdksArgs) -> Result<Report> {
    match fetch_report(args.clone()).await {
        Ok(report) => Ok(report),
        Err(error) if needs_login(&error) => {
            clear_saved_session()?;
            ensure_session()?;
            fetch_report(args).await
        }
        Err(error) => Err(error),
    }
}

pub fn ensure_session() -> Result<()> {
    if has_saved_session()? {
        return Ok(());
    }
    let cookie = capture_portal_cookie()?
        .ok_or_else(|| anyhow::anyhow!("Portal window was closed before sign-in completed."))?;
    save_session(&cookie)
}

pub fn has_saved_session() -> Result<bool> {
    Ok(load_session()?.is_some())
}

/// Forget the portal login stored in the system keychain.
pub fn clear_saved_session() -> Result<()> {
    keychain_delete(KEY_SESSION)
}

pub fn login() -> Result<()> {
    println!(
        "Sign in to Ar-Ge Portal in the window that opens. The window closes once the session cookie is saved."
    );
    let cookie = capture_portal_cookie()?.ok_or_else(|| {
        anyhow::anyhow!(
            "Window closed before a portal session was available. Sign in, then run `jf pdks login` again."
        )
    })?;
    save_session(&cookie)?;
    println!("Portal session saved. Run `jf pdks` to calculate hours.");
    Ok(())
}

pub fn needs_login(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("No saved portal session")
        || message.contains("Portal session expired")
        || message.contains("No portal term was returned")
}

fn capture_portal_cookie() -> Result<Option<String>> {
    let logged_in = Arc::new(AtomicBool::new(false));
    let logged_in_ipc = logged_in.clone();
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Jotmate · Ar-Ge Portal")
        .with_inner_size(tao::dpi::LogicalSize::new(1120.0, 800.0))
        .build(&event_loop)
        .context("Could not open the Ar-Ge Portal window")?;
    let mut context = WebContext::new(dirs::cache_dir().map(|p| p.join("jotmate/pdks-webview")));
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(BASE_URL)
        .with_initialization_script(LOGIN_PROBE)
        .with_ipc_handler(move |request| {
            if serde_json::from_str::<Value>(request.body())
                .ok()
                .and_then(|value| value.get("loggedIn")?.as_bool())
                == Some(true)
            {
                logged_in_ipc.store(true, Ordering::SeqCst);
            }
        })
        .build(&window)
        .context("Could not start the Ar-Ge Portal view")?;

    let mut cookie_header = None;
    let mut close_deadline = None;
    event_loop.run_return(|event, _, control_flow| {
        if close_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            *control_flow = ControlFlow::Exit;
            return;
        }
        *control_flow = close_deadline
            .map(ControlFlow::WaitUntil)
            .unwrap_or(ControlFlow::Wait);
        if logged_in.load(Ordering::SeqCst) && cookie_header.is_none() {
            match webview.cookies() {
                Ok(cookies) => {
                    let header = format_cookie_header(&cookies);
                    if !header.is_empty() {
                        cookie_header = Some(header);
                        window.set_visible(false);
                        close_deadline = Some(Instant::now() + Duration::from_millis(50));
                        *control_flow = ControlFlow::WaitUntil(close_deadline.unwrap());
                        return;
                    }
                }
                Err(error) => {
                    eprintln!("Could not read portal cookies: {error}");
                }
            }
        }
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            window.set_visible(false);
            close_deadline = Some(Instant::now() + Duration::from_millis(50));
            *control_flow = ControlFlow::WaitUntil(close_deadline.unwrap());
        }
    });
    Ok(cookie_header)
}

fn format_cookie_header(cookies: &[wry::cookie::Cookie<'_>]) -> String {
    cookies
        .iter()
        .filter(|cookie| !cookie.name().is_empty() && !cookie.value().is_empty())
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        .collect::<Vec<_>>()
        .join("; ")
}

async fn fetch_with_saved_session(args: &PdksArgs, week: NaiveDate) -> Result<Vec<Value>> {
    let cookie = load_session()?.ok_or_else(|| anyhow::anyhow!("No saved portal session."))?;
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)",
        )
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Could not build the PDKS HTTP client")?;

    let periods = post_form(
        &client,
        &cookie,
        "/ProjeSabit/FirmaDonem",
        &[("Firma_Id", args.company_id.to_string())],
    )
    .await?;
    let period_ids = period_ids_for_week(&periods, week, args.period_id)?;
    let mut reports = Vec::with_capacity(period_ids.len());
    for period in period_ids {
        let personnel = post_form(
            &client,
            &cookie,
            "/PersonelSabit/PersonelPDKSDonemdeCalisanPersonelListesi",
            &[
                ("Donem_Id", period.to_string()),
                ("Firma_Id", args.company_id.to_string()),
            ],
        )
        .await?;
        let people = collect_ids(&personnel);
        let person = match args.personnel_id {
            Some(id) => id,
            None if people.len() == 1 => people[0],
            None if people.is_empty() => {
                anyhow::bail!("No personnel records were returned for portal term {period}")
            }
            None => anyhow::bail!(
                "This account sees {} employees. Re-run with --personnel-id <id>. Available: {}",
                people.len(),
                people
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        reports.push(
            post_form(
                &client,
                &cookie,
                "/Personel/PersonelGirisCikis/PersonelPdksGirisCikisListe",
                &[
                    ("page", "1".into()),
                    ("rp", "1000".into()),
                    ("sortname", "Id".into()),
                    ("sortorder", "asc".into()),
                    ("query", String::new()),
                    ("qtype", String::new()),
                    ("Firma_Id", args.company_id.to_string()),
                    ("Donem_Id", period.to_string()),
                    ("Personel_Id", person.to_string()),
                ],
            )
            .await?,
        );
    }
    Ok(reports)
}

async fn post_form(
    client: &reqwest::Client,
    cookie: &str,
    path: &str,
    fields: &[(&str, String)],
) -> Result<Value> {
    let url = format!("{BASE_URL}{path}");
    let response = client
        .post(&url)
        .header("Cookie", cookie)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Origin", BASE_URL)
        .header("Referer", format!("{BASE_URL}/"))
        .form(fields)
        .send()
        .await
        .with_context(|| format!("Request to {path} failed"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("Could not read {path} response"))?;
    if !status.is_success() || looks_like_html(&text) {
        anyhow::bail!("Portal session expired. Run `jf pdks login` and try again.");
    }
    serde_json::from_str(&text).with_context(|| format!("Unexpected response from {path}"))
}

fn looks_like_html(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("<!") || trimmed.to_ascii_lowercase().starts_with("<html")
}

fn collect_ids(value: &Value) -> Vec<u64> {
    let mut ids = BTreeSet::new();
    collect_ids_into(value, &mut ids);
    ids.into_iter().collect()
}

fn collect_ids_into(value: &Value, ids: &mut BTreeSet<u64>) {
    match value {
        Value::Array(values) => values.iter().for_each(|v| collect_ids_into(v, ids)),
        Value::Object(map) => {
            for (key, nested) in map {
                if matches!(key.as_str(), "id" | "Id" | "value" | "Value") {
                    if let Some(id) = parse_id(nested) {
                        ids.insert(id);
                    }
                }
                collect_ids_into(nested, ids);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
struct PortalPeriod {
    id: u64,
    dates: Vec<NaiveDate>,
    label: String,
}

/// Select every portal period which overlaps the requested Monday–Sunday week.
/// A week can cross a calendar-month boundary, so querying just one period loses
/// the days on the other side of the boundary.
fn period_ids_for_week(
    payload: &Value,
    week: NaiveDate,
    explicitly_requested: Option<u64>,
) -> Result<Vec<u64>> {
    if let Some(period) = explicitly_requested {
        return Ok(vec![period]);
    }

    let period_ids = collect_ids(payload);
    if period_ids.is_empty() {
        anyhow::bail!("No portal term was returned. Sign in again with `jf pdks login`.")
    }
    let end = week + chrono::Duration::days(6);
    let matched: Vec<u64> = collect_portal_periods(payload)
        .into_iter()
        .filter(|period| period_matches_week(period, week, end))
        .map(|period| period.id)
        .collect();
    if !matched.is_empty() {
        return Ok(matched);
    }

    // Older accounts sometimes return select options without their date label.
    // Period ids are sequential month-by-month in that response; this fallback
    // lets historical navigation still work while preferring explicit dates above.
    let latest = *period_ids.last().expect("checked non-empty");
    let today = Local::now().date_naive();
    let month_delta = months_between(week, today);
    let inferred = latest
        .checked_sub(month_delta)
        .filter(|id| period_ids.contains(id));
    inferred.map(|id| vec![id]).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not identify the portal period for {}. Run `jf pdks --period-id <id>` for this week.",
            week.format("%d.%m.%Y")
        )
    })
}

fn collect_portal_periods(value: &Value) -> Vec<PortalPeriod> {
    let mut periods = Vec::new();
    collect_portal_periods_into(value, &mut periods);
    periods
}

fn collect_portal_periods_into(value: &Value, periods: &mut Vec<PortalPeriod>) {
    if let Some(period) = portal_period_from_object(value) {
        periods.push(period);
        return;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect_portal_periods_into(value, periods)),
        Value::Object(map) => map
            .values()
            .for_each(|value| collect_portal_periods_into(value, periods)),
        _ => {}
    }
}

fn portal_period_from_object(value: &Value) -> Option<PortalPeriod> {
    let Value::Object(map) = value else {
        return None;
    };
    let id = map.iter().find_map(|(key, value)| {
        matches!(key.as_str(), "Id" | "id" | "Value" | "value")
            .then(|| parse_id(value))
            .flatten()
    })?;
    let strings: Vec<&str> = map.values().filter_map(Value::as_str).collect();
    let dates = strings
        .iter()
        .flat_map(|text| dates_in_text(text))
        .collect();
    Some(PortalPeriod {
        id,
        dates,
        label: strings.join(" ").to_lowercase(),
    })
}

fn period_matches_week(period: &PortalPeriod, week: NaiveDate, end: NaiveDate) -> bool {
    if let (Some(first), Some(last)) = (period.dates.iter().min(), period.dates.iter().max()) {
        return *first <= end && *last >= week;
    }
    (0..=6)
        .map(|day| week + chrono::Duration::days(day))
        .any(|day| {
            let month = day.month();
            let year = day.year();
            let month_name = [
                "ocak", "şubat", "mart", "nisan", "mayıs", "haziran", "temmuz", "ağustos", "eylül",
                "ekim", "kasım", "aralık",
            ][month as usize - 1];
            period.label.contains(month_name)
                && (period.label.contains(&year.to_string())
                    || period.label.contains(&format!("{year}-{month:02}"))
                    || period.label.contains(&format!("{month:02}.{year}")))
        })
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    let value = value.replace("<b>", " ").replace("</b>", " ");
    ["%d.%m.%Y", "%Y-%m-%d", "%d/%m/%Y"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value.trim(), format).ok())
        .or_else(|| parse_timestamp(&value).map(|date_time| date_time.date()))
}

fn dates_in_text(value: &str) -> Vec<NaiveDate> {
    let direct = parse_date(value);
    if direct.is_some() {
        return direct.into_iter().collect();
    }
    value
        .split(|character: char| {
            !(character.is_ascii_digit() || matches!(character, '.' | '/' | '-'))
        })
        .filter_map(parse_date)
        .collect()
}

fn months_between(earlier: NaiveDate, later: NaiveDate) -> u64 {
    let earlier_month = earlier.year() * 12 + earlier.month() as i32;
    let later_month = later.year() * 12 + later.month() as i32;
    (later_month - earlier_month).max(0) as u64
}

fn parse_id(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) if text.chars().all(|c| c.is_ascii_digit()) && !text.is_empty() => {
            text.parse().ok()
        }
        _ => None,
    }
}

fn load_session() -> Result<Option<String>> {
    keychain_get(KEY_SESSION)
}

fn save_session(cookie: &str) -> Result<()> {
    keychain_set(KEY_SESSION, cookie)
}

fn keychain_get(account: &str) -> Result<Option<String>> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w",
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .context("security CLI not found")?;
    if out.status.success() {
        let value = String::from_utf8(out.stdout).context("keychain returned non-UTF8 data")?;
        let trimmed = value.trim().to_string();
        return Ok((!trimmed.is_empty()).then_some(trimmed));
    }
    if out.status.code() == Some(44) {
        return Ok(None);
    }
    anyhow::bail!(
        "Could not read the PDKS session from Keychain: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    )
}

fn keychain_set(account: &str, value: &str) -> Result<()> {
    let _ = keychain_delete(account);
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w",
            value,
            "-U",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("security CLI not found")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Could not save the PDKS session to Keychain")
    }
}

fn keychain_delete(account: &str) -> Result<()> {
    let status = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("security CLI not found")?;
    if status.success() || status.code() == Some(44) {
        Ok(())
    } else {
        anyhow::bail!("Could not clear the PDKS session from Keychain")
    }
}

fn parse_punches(payload: &Value) -> Vec<Punch> {
    let mut punches = Vec::new();
    collect_punch_records(payload, &mut punches);
    if punches.is_empty() {
        // Keep accepting older portal payload shapes that omit the status field.
        let mut timestamps = Vec::new();
        collect_timestamps(payload, &mut timestamps);
        punches = timestamps
            .into_iter()
            .map(|at| Punch {
                at,
                state: PunchState::Unknown,
            })
            .collect();
    }
    punches.sort_by_key(|punch| punch.at);
    punches.dedup_by_key(|punch| punch.at);
    punches
}

fn collect_punch_records(value: &Value, punches: &mut Vec<Punch>) {
    if let Some(punch) = punch_from_row(value) {
        punches.push(punch);
        return;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect_punch_records(value, punches)),
        Value::Object(map) => map
            .values()
            .for_each(|value| collect_punch_records(value, punches)),
        _ => {}
    }
}

fn punch_from_row(value: &Value) -> Option<Punch> {
    let Value::Object(map) = value else {
        return None;
    };
    let mut strings = Vec::new();
    for value in map.values() {
        match value {
            Value::String(text) => strings.push(text.as_str()),
            Value::Array(values) => strings.extend(values.iter().filter_map(Value::as_str)),
            _ => {}
        }
    }
    let at = strings.iter().find_map(|text| parse_timestamp(text))?;
    let state = map
        .iter()
        .find_map(|(key, value)| {
            matches!(key.as_str(), "Durum" | "Durumu" | "durum" | "durumu")
                .then(|| value.as_str())
                .flatten()
        })
        .and_then(classify_punch_state)
        .or_else(|| strings.iter().find_map(|text| classify_punch_state(text)))
        .unwrap_or(PunchState::Unknown);
    Some(Punch { at, state })
}

fn classify_punch_state(value: &str) -> Option<PunchState> {
    let value = value.to_lowercase();
    if value.contains("çıkış") || value.contains("cikis") || value.contains("exit") {
        Some(PunchState::Exit)
    } else if value.contains("giriş") || value.contains("giris") || value.contains("entry") {
        Some(PunchState::Entry)
    } else {
        None
    }
}

fn collect_timestamps(value: &Value, timestamps: &mut Vec<NaiveDateTime>) {
    match value {
        Value::String(text) => {
            if let Some(at) = parse_timestamp(text) {
                timestamps.push(at);
            }
        }
        Value::Array(values) => values
            .iter()
            .for_each(|v| collect_timestamps(v, timestamps)),
        Value::Object(map) => map.values().for_each(|v| collect_timestamps(v, timestamps)),
        _ => {}
    }
}

fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
    let value = value.replace("<b>", " ").replace("</b>", "");
    ["%d.%m.%Y %H:%M:%S", "%d.%m.%Y %H:%M", "%Y-%m-%dT%H:%M:%S"]
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(value.trim(), format).ok())
}

fn build_report(week: NaiveDate, target_hours: f64, punches: &[Punch]) -> Report {
    build_report_at(week, target_hours, punches, Local::now().naive_local())
}

fn build_report_at(
    week: NaiveDate,
    target_hours: f64,
    punches: &[Punch],
    now: NaiveDateTime,
) -> Report {
    let end = week + chrono::Duration::days(6);
    let mut days: BTreeMap<NaiveDate, Vec<&Punch>> = BTreeMap::new();
    for punch in punches
        .iter()
        .filter(|p| p.at.date() >= week && p.at.date() <= end)
    {
        days.entry(punch.at.date()).or_default().push(punch);
    }
    let days: Vec<ReportDay> = days
        .into_iter()
        .filter_map(|(date, entries)| {
            let has_entry = entries.iter().any(|punch| punch.state == PunchState::Entry);
            let first = entries
                .iter()
                .find(|punch| punch.state == PunchState::Entry)
                .or_else(|| entries.first())?;
            let last_recorded = entries.last()?;
            let is_still_in_office = has_entry
                && date == now.date()
                && (last_recorded.state == PunchState::Entry || now.hour() < 15);
            let last = if is_still_in_office {
                now
            } else {
                last_recorded.at
            };
            Some(ReportDay {
                date,
                first: first.at,
                last,
                worked_secs: (last - first.at).num_seconds().max(0),
                is_still_in_office,
            })
        })
        .collect();
    let worked_secs = days.iter().map(|day| day.worked_secs).sum();
    Report {
        week,
        target_hours,
        worked_secs,
        days,
    }
}

fn print_report(report: &Report) {
    let end = report.week + chrono::Duration::days(6);
    println!(
        "PDKS  {} – {}",
        report.week.format("%d.%m.%Y"),
        end.format("%d.%m.%Y")
    );
    for day in &report.days {
        let hours = day.worked_secs as f64 / 3600.0;
        println!(
            "{}  {} – {}{}  {}  ({}){}",
            day.date.format("%a %d.%m"),
            day.first.format("%H:%M"),
            if day.is_still_in_office {
                "now".to_string()
            } else {
                day.last.format("%H:%M").to_string()
            },
            if day.is_still_in_office {
                " (in office)"
            } else {
                ""
            },
            format_hours(hours),
            crate::time::compute::format_hours_signed(hours - PDKS_DAILY_TARGET_HOURS),
            if day.is_still_in_office {
                format!("  leave at {}", day.target_exit().format("%H:%M"))
            } else {
                String::new()
            },
        );
    }
    let worked = report.worked_secs as f64 / 3600.0;
    let balance = worked - report.target_hours;
    println!(
        "\nToplam: {} / {}  ({})",
        format_hours(worked),
        format_hours(report.target_hours),
        crate::time::compute::format_hours_signed(balance)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn extracts_dates_from_named_and_grid_cells() {
        let value: Value = serde_json::json!({"rows":[
            {"Tarih":"20.10.2026 <b>09:15:00"},
            {"cell":["21.10.2026 18:00:00", "not a timestamp"]}
        ]});
        let punches = parse_punches(&value);
        assert_eq!(punches.len(), 2);
        assert_eq!(punches[0].at.date().day(), 20);
    }

    #[test]
    fn accepts_an_empty_period_as_a_zero_hour_week() {
        let week = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        let report = build_report(week, PDKS_WEEKLY_TARGET_HOURS, &parse_punches(&Value::Null));
        assert!(report.days.is_empty());
        assert_eq!(report.worked_secs, 0);
    }

    #[test]
    fn calculates_week_start() {
        assert_eq!(
            get_week_start_monday(NaiveDate::from_ymd_opt(2026, 10, 22).unwrap()),
            NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()
        );
    }

    #[test]
    fn collects_nested_period_ids_in_order() {
        let value = serde_json::json!([
            {"Id": "3", "name": "older"},
            {"id": 12, "nested": {"Value": "7"}}
        ]);
        assert_eq!(collect_ids(&value), vec![3, 7, 12]);
    }

    #[test]
    fn selects_all_periods_for_a_week_that_crosses_months() {
        let periods = serde_json::json!([
            {"Id": 285, "Text": "01.09.2026 - 30.09.2026"},
            {"Id": 286, "Text": "01.10.2026 - 31.10.2026"}
        ]);
        let week = NaiveDate::from_ymd_opt(2026, 9, 28).unwrap();
        assert_eq!(
            period_ids_for_week(&periods, week, None).unwrap(),
            vec![285, 286]
        );
    }

    #[test]
    fn selects_the_previous_months_period_for_a_historical_week() {
        let periods = serde_json::json!([
            {"Id": 284, "Text": "01.08.2026 - 31.08.2026"},
            {"Id": 285, "Text": "01.09.2026 - 30.09.2026"},
            {"Id": 286, "Text": "01.10.2026 - 31.10.2026"}
        ]);
        let week = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        assert_eq!(
            period_ids_for_week(&periods, week, None).unwrap(),
            vec![285]
        );
    }

    #[test]
    fn detects_html_login_pages() {
        assert!(looks_like_html("  <!DOCTYPE html><html>"));
        assert!(looks_like_html("<HTML lang=tr>"));
        assert!(!looks_like_html("{\"rows\":[]}"));
    }

    #[test]
    fn treats_a_current_final_entry_as_an_open_workday() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        let entry = day.and_hms_opt(9, 0, 0).unwrap();
        let now = day.and_hms_opt(17, 30, 0).unwrap();
        let report = build_report_at(
            get_week_start_monday(day),
            45.0,
            &[Punch {
                at: entry,
                state: PunchState::Entry,
            }],
            now,
        );
        assert!(report.days[0].is_still_in_office);
        assert_eq!(report.days[0].worked_secs, 8 * 3600 + 30 * 60);
    }

    #[test]
    fn keeps_a_final_exit_as_the_day_end() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        let report = build_report_at(
            get_week_start_monday(day),
            45.0,
            &[
                Punch {
                    at: day.and_hms_opt(9, 0, 0).unwrap(),
                    state: PunchState::Entry,
                },
                Punch {
                    at: day.and_hms_opt(18, 0, 0).unwrap(),
                    state: PunchState::Exit,
                },
            ],
            day.and_hms_opt(19, 0, 0).unwrap(),
        );
        assert!(!report.days[0].is_still_in_office);
        assert_eq!(report.days[0].worked_secs, 9 * 3600);
    }

    #[test]
    fn treats_an_early_exit_before_three_as_still_in_office() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        let report = build_report_at(
            get_week_start_monday(day),
            45.0,
            &[
                Punch {
                    at: day.and_hms_opt(9, 0, 0).unwrap(),
                    state: PunchState::Entry,
                },
                Punch {
                    at: day.and_hms_opt(12, 0, 0).unwrap(),
                    state: PunchState::Exit,
                },
            ],
            day.and_hms_opt(14, 30, 0).unwrap(),
        );
        assert!(report.days[0].is_still_in_office);
        assert_eq!(report.days[0].worked_secs, 5 * 3600 + 30 * 60);
    }
}
