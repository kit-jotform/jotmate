use ratatui::layout::{Constraint, Direction, Layout, Rect};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Canonical UI width — matches the full header (icon 14 + gap 2 + logo 63).
pub const UI_WIDTH: u16 = 79;

// ── Horizontal alignment ──────────────────────────────────────────────────────

pub enum HAlign {
    /// Flush with the left edge of the UI area.
    Left,
    /// Horizontally centered within `UI_WIDTH`.
    Center,
}

// ── Widget descriptor ─────────────────────────────────────────────────────────

pub struct Widget {
    pub width: u16,
    pub halign: HAlign,
}

impl Widget {
    pub fn anon(width: u16, halign: HAlign) -> Self {
        Self { width, halign }
    }
}

// ── Layout engine (horizontal) ────────────────────────────────────────────────

pub struct LayoutEngine {
    ui_width: u16,
    base_x: u16,
}

impl LayoutEngine {
    pub fn new(base_x: u16) -> Self {
        Self {
            ui_width: UI_WIDTH,
            base_x,
        }
    }

    pub fn place(&self, w: &Widget, row: Rect) -> Rect {
        let x = match w.halign {
            HAlign::Left => self.base_x,
            HAlign::Center => self.base_x + self.ui_width.saturating_sub(w.width) / 2,
        };
        Rect {
            x,
            width: self.ui_width.min(w.width),
            ..row
        }
    }

    pub fn center(&self, width: u16, row: Rect) -> Rect {
        let x = self.base_x + self.ui_width.saturating_sub(width) / 2;
        Rect {
            x,
            width: self.ui_width.min(width),
            ..row
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
