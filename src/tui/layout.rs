use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Icon (14) + gap (2) + logo (63).
pub const UI_WIDTH: u16 = 79;

/// Below this terminal height we drop decorative chrome (logos, taglines)
/// to keep the actual content visible.
pub const COMPACT_HEIGHT: u16 = 24;

pub fn is_compact(area: Rect) -> bool {
    area.height < COMPACT_HEIGHT
}

pub enum HAlign {
    Left,
    Center,
}

pub struct Widget {
    pub width: u16,
    pub halign: HAlign,
}

impl Widget {
    pub fn anon(width: u16, halign: HAlign) -> Self {
        Self { width, halign }
    }
}

pub struct LayoutEngine {
    ui_width: u16,
    base_x: u16,
    frame_right: u16,
}

impl LayoutEngine {
    pub fn new(frame: Rect) -> Self {
        let ui_width = UI_WIDTH.min(frame.width);
        Self {
            ui_width,
            base_x: frame.x,
            frame_right: frame.x + frame.width,
        }
    }

    pub fn clamp_area(&self, area: Rect) -> Rect {
        Rect {
            x: self.base_x,
            width: self.ui_width,
            ..area
        }
    }

    pub fn place(&self, w: &Widget, row: Rect) -> Rect {
        let x = match w.halign {
            HAlign::Left => self.base_x,
            HAlign::Center => self.base_x + self.ui_width.saturating_sub(w.width) / 2,
        };
        self.clip(x, self.ui_width.min(w.width), row)
    }

    pub fn center(&self, width: u16, row: Rect) -> Rect {
        let x = self.base_x + self.ui_width.saturating_sub(width) / 2;
        self.clip(x, self.ui_width.min(width), row)
    }

    fn clip(&self, x: u16, width: u16, row: Rect) -> Rect {
        let x = x.min(self.frame_right);
        let right = (x + width).min(self.frame_right);
        Rect {
            x,
            width: right - x,
            ..row
        }
    }
}

pub struct ScreenLayout {
    rows: Vec<(&'static str, Constraint)>,
    margin: u16,
}

impl ScreenLayout {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            margin: 0,
        }
    }

    /// Add a row with a fixed height. A height of 0 collapses the row entirely
    /// (so screens can hide chrome conditionally without changing row names).
    /// To make a row fill remaining space, use [`ScreenLayout::fill`].
    pub fn row(mut self, name: &'static str, height: u16) -> Self {
        self.rows.push((name, Constraint::Length(height)));
        self
    }

    /// Add a row that fills the remaining vertical space.
    pub fn fill(mut self, name: &'static str) -> Self {
        self.rows.push((name, Constraint::Min(0)));
        self
    }

    pub fn margin(mut self, m: u16) -> Self {
        self.margin = m;
        self
    }

    pub fn split(&self, area: Rect) -> RowMap {
        let constraints: Vec<Constraint> = self.rows.iter().map(|(_, c)| *c).collect();

        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .margin(self.margin)
            .split(area);

        let named: Vec<(&'static str, Rect)> = self
            .rows
            .iter()
            .zip(rects.iter())
            .map(|((name, _), r)| (*name, *r))
            .collect();

        RowMap(named)
    }
}

pub struct RowMap(Vec<(&'static str, Rect)>);

impl RowMap {
    pub fn get(&self, name: &str) -> Rect {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("RowMap: unknown row name '{name}'"))
    }
}
