//! Shared draw helpers: hint spans, common list-item builders, header renderer,
//! confirm-dialog overlay, and layout/constant helpers used by every sub-screen.
//!
//! Everything here is `pub(in crate::tui::draw)` so sibling draw modules can
//! import from `super::common::*` (or, via re-exports in `draw/mod.rs`, from
//! `super::*`).

mod dialog;
mod header;
mod hints;
mod items;

pub(in crate::tui::draw) use dialog::draw_confirm_dialog;
pub(in crate::tui::draw) use header::draw_screen_header;
pub(in crate::tui::draw) use hints::{
    hint_confirm_cancel, hint_input_confirm, hint_muted, hint_navigate_action, HINT_CYCLE_VALUE,
    HINT_RETURN_TO_MENU,
};
pub(in crate::tui::draw) use items::{
    back_item, blank_item, divider_item, field_state, inline_field_item, link_item, separator_item,
    sub_link_item, toggle_item, FieldState, FIELD_LABEL_W, FIELD_LABEL_W_TZ,
};

use chrono::NaiveDate;
use ratatui::layout::Rect;

use crate::tui::layout::{RowMap, ScreenLayout};

pub(in crate::tui::draw) fn fmt_date(d: NaiveDate) -> String {
    d.format("%d-%m-%Y").to_string()
}

pub(in crate::tui::draw) const DIVIDER_WIDTH: u16 = 53;
/// Visible char width of a settings-screen section separator line.
pub(in crate::tui::draw) const SEPARATOR_WIDTH: usize = 46;

/// Standard sub-screen layout: small logo, title bar, divider, then a fill
/// area for lists. Used by every screen except MainMenu and SyncProgress.
pub(in crate::tui::draw) fn sub_screen_layout(area: Rect) -> RowMap {
    ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("list", 0)
        .margin(1)
        .split(area)
}
