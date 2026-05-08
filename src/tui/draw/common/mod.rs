mod dialog;
mod header;
mod hints;
mod items;
mod scroll_table;

pub(in crate::tui::draw) use dialog::draw_confirm_dialog;
pub(in crate::tui::draw) use header::draw_screen_header;
pub(in crate::tui::draw) use hints::{
    hint_confirm_cancel, hint_input_confirm, hint_muted, hint_navigate_action, HINT_CYCLE_VALUE,
    HINT_RETURN_TO_MENU,
};
pub(in crate::tui::draw) use items::{
    back_item, blank_item, del_item, divider_item, field_state, inline_field_item, link_item,
    separator_item, sub_link_item, toggle_item, FieldState, FIELD_LABEL_W, FIELD_LABEL_W_TZ,
};
pub(in crate::tui::draw) use scroll_table::draw_scroll_table;

use chrono::NaiveDate;
use ratatui::layout::Rect;

use crate::tui::layout::{is_compact, LayoutEngine, RowMap, ScreenLayout};

pub(in crate::tui::draw) fn fmt_date(d: NaiveDate) -> String {
    d.format("%d-%m-%Y").to_string()
}

pub(in crate::tui::draw) fn fmt_hours(h: f64) -> String {
    if h.fract() == 0.0 {
        format!("{}h", h as u32)
    } else {
        format!("{h}h")
    }
}

pub(in crate::tui::draw) fn inset_rect(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x + margin,
        width: area.width.saturating_sub(margin * 2),
        ..area
    }
}

pub(in crate::tui::draw) const DIVIDER_WIDTH: u16 = 53;
pub(in crate::tui::draw) const SEPARATOR_WIDTH: usize = 46;

pub(in crate::tui::draw) fn sub_screen_layout(area: Rect) -> RowMap {
    let compact = is_compact(area);
    let logo_h = if compact { 0 } else { 3 };
    let blank1_h = if compact { 0 } else { 1 };
    ScreenLayout::new()
        .row("logo", logo_h)
        .row("blank1", blank1_h)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .fill("list")
        .margin(1)
        .split(area)
}

pub(in crate::tui::draw) fn sub_screen_setup(f: &ratatui::Frame) -> (Rect, LayoutEngine, RowMap) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));
    (area, engine, layout)
}
