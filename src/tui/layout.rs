use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Canonical UI width — matches the full header (icon 14 + gap 2 + logo 63).
pub const UI_WIDTH: u16 = 79;

// ── Widget identity ───────────────────────────────────────────────────────────

pub type WidgetId = u32;

// ── Horizontal alignment ──────────────────────────────────────────────────────

#[allow(dead_code)]
pub enum HAlign {
    /// Flush with the left edge of the UI area.
    Left,
    /// Flush with the right edge of the UI area.
    Right,
    /// Horizontally centered within `UI_WIDTH`.
    Center,
    /// Absolute offset from `base_x`.
    Custom(u16),
    /// Same left edge as a previously placed widget.
    AlignedWith(WidgetId),
    /// Horizontally centered relative to a previously placed widget's span.
    CenteredWith(WidgetId),
    /// Placed immediately to the right of a widget, with an optional gap.
    RightOf(WidgetId, u16),
    /// Placed immediately to the left of a widget, with an optional gap.
    LeftOf(WidgetId, u16),
}

// ── Widget descriptor ─────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct Widget {
    pub id: Option<WidgetId>,
    pub width: u16,
    pub height: u16,
    pub halign: HAlign,
}

impl Widget {
    /// Convenience constructor for widgets that don't need to be referenced by others.
    pub fn anon(width: u16, height: u16, halign: HAlign) -> Self {
        Self {
            id: None,
            width,
            height,
            halign,
        }
    }

    #[allow(dead_code)]
    pub fn named(id: WidgetId, width: u16, height: u16, halign: HAlign) -> Self {
        Self {
            id: Some(id),
            width,
            height,
            halign,
        }
    }
}

// ── Layout engine (horizontal) ────────────────────────────────────────────────

pub struct LayoutEngine {
    ui_width: u16,
    base_x: u16,
    placed: HashMap<WidgetId, Rect>,
}

impl LayoutEngine {
    pub fn new(base_x: u16) -> Self {
        Self {
            ui_width: UI_WIDTH,
            base_x,
            placed: HashMap::new(),
        }
    }

    /// Resolve horizontal alignment for `w` within `row`, store if named, return the `Rect`.
    pub fn place(&mut self, w: &Widget, row: Rect) -> Rect {
        let x = self.resolve_x(w, &w.halign);
        let width = self.ui_width.min(w.width);
        let rect = Rect { x, width, ..row };
        if let Some(id) = w.id {
            self.placed.insert(id, rect);
        }
        rect
    }

    /// Convenience: center an anonymous widget within `UI_WIDTH`. No HashMap write.
    pub fn center(&self, width: u16, row: Rect) -> Rect {
        let x = self.base_x + self.ui_width.saturating_sub(width) / 2;
        let width = self.ui_width.min(width);
        Rect { x, width, ..row }
    }

    fn resolve_x(&self, w: &Widget, halign: &HAlign) -> u16 {
        match halign {
            HAlign::Left => self.base_x,
            HAlign::Right => self.base_x + self.ui_width.saturating_sub(w.width),
            HAlign::Center => self.base_x + self.ui_width.saturating_sub(w.width) / 2,
            HAlign::Custom(off) => self.base_x + off,
            HAlign::AlignedWith(id) => self.placed.get(id).map(|r| r.x).unwrap_or(self.base_x),
            HAlign::CenteredWith(id) => {
                let anchor = self.placed.get(id).copied().unwrap_or(Rect {
                    x: self.base_x,
                    width: self.ui_width,
                    y: 0,
                    height: 1,
                });
                anchor.x + anchor.width.saturating_sub(w.width) / 2
            }
            HAlign::RightOf(id, gap) => {
                let anchor = self.placed.get(id).copied().unwrap_or(Rect {
                    x: self.base_x,
                    width: 0,
                    y: 0,
                    height: 1,
                });
                anchor.x + anchor.width + gap
            }
            HAlign::LeftOf(id, gap) => {
                let anchor = self.placed.get(id).copied().unwrap_or(Rect {
                    x: self.base_x + self.ui_width,
                    width: 0,
                    y: 0,
                    height: 1,
                });
                anchor.x.saturating_sub(w.width + gap)
            }
        }
    }
}

// ── Screen layout (vertical rows) ─────────────────────────────────────────────

pub struct ScreenLayout {
    rows: Vec<(&'static str, u16)>, // (name, height); 0 → Min(0)
    margin: u16,
}

impl ScreenLayout {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            margin: 0,
        }
    }

    /// Add a named row. `height = 0` becomes `Constraint::Min(0)`.
    pub fn row(mut self, name: &'static str, height: u16) -> Self {
        self.rows.push((name, height));
        self
    }

    pub fn margin(mut self, m: u16) -> Self {
        self.margin = m;
        self
    }

    pub fn split(&self, area: Rect) -> RowMap {
        let constraints: Vec<Constraint> = self
            .rows
            .iter()
            .map(|&(_, h)| {
                if h == 0 {
                    Constraint::Min(0)
                } else {
                    Constraint::Length(h)
                }
            })
            .collect();

        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .margin(self.margin)
            .split(area);

        let named: Vec<(&'static str, Rect)> = self
            .rows
            .iter()
            .zip(rects.iter())
            .map(|(&(name, _), &r)| (name, r))
            .collect();

        RowMap(named)
    }
}

// ── RowMap ────────────────────────────────────────────────────────────────────

pub struct RowMap(Vec<(&'static str, Rect)>);

impl RowMap {
    /// Get the `Rect` for a named row. Panics with a clear message if the name is unknown.
    pub fn get(&self, name: &str) -> Rect {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("RowMap: unknown row name '{name}'"))
    }
}
