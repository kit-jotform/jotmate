use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub const LOGO: [&str; 6] = [
    "     ██╗ ██████╗ ████████╗███╗   ███╗ █████╗ ████████╗███████╗",
    "     ██║██╔═══██╗╚══██╔══╝████╗ ████║██╔══██╗╚══██╔══╝██╔════╝",
    "     ██║██║   ██║   ██║   ██╔████╔██║███████║   ██║   █████╗  ",
    "██   ██║██║   ██║   ██║   ██║╚██╔╝██║██╔══██║   ██║   ██╔══╝  ",
    "╚█████╔╝╚██████╔╝   ██║   ██║ ╚═╝ ██║██║  ██║   ██║   ███████╗",
    " ╚════╝  ╚═════╝    ╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝   ╚═╝   ╚══════╝",
];

pub const LOGO_SMALL: [&str; 3] = [
    " ╦╔═╗╔╦╗╔╦╗╔═╗╔╦╗╔═╗",
    "║║ ║  ║ ║║║╠═╣ ║ ║╣ ",
    "╚╝╚═╝ ╩ ╩ ╩╩ ╩ ╩ ╚═╝",
];

// (col, row, char, fg_256, bg_256) — None = terminal default
type IconCell = (u16, u16, char, Option<u8>, Option<u8>);

#[rustfmt::skip]
const ICON_CELLS: &[IconCell] = &[
    (0,0,'▗',Some(102),None),(1,0,'▅',Some(244),None),(2,0,'▅',Some(243),None),
    (3,0,'▃',Some(244),None),(4,0,' ',Some(244),None),(5,0,' ',Some(244),None),
    (6,0,' ',Some(244),None),(7,0,' ',Some(244),None),(8,0,' ',Some(244),None),
    (9,0,'▕',Some(235),None),(10,0,'▃',Some(243),None),(11,0,'▅',None,None),
    (12,0,'▅',None,None),(13,0,'▖',Some(246),None),
    (0,1,'▍',Some(16),Some(244)),(1,1,'▎',Some(233),Some(173)),(2,1,'▞',Some(173),Some(167)),
    (3,1,'▆',Some(173),Some(237)),(4,1,'▃',Some(179),Some(241)),(5,1,'▔',Some(16),Some(137)),
    (6,1,'▄',Some(179),Some(243)),(7,1,'▄',Some(179),Some(243)),(8,1,'▔',Some(16),Some(102)),
    (9,1,'▃',Some(179),Some(242)),(10,1,'▆',Some(173),Some(238)),(11,1,'▃',Some(167),Some(173)),
    (12,1,'▋',Some(173),Some(233)),(13,1,'▌',Some(145),Some(233)),
    (0,2,'▍',Some(16),Some(244)),(1,2,'▌',Some(237),Some(173)),(2,2,'▂',Some(179),Some(173)),
    (3,2,'▄',Some(180),Some(173)),(4,2,'▌',Some(179),Some(180)),(5,2,'▊',Some(179),Some(150)),
    (6,2,'▎',Some(180),Some(179)),(7,2,'▞',Some(179),Some(150)),(8,2,'▌',Some(180),Some(215)),
    (9,2,'▘',Some(150),Some(179)),(10,2,'▁',Some(180),Some(173)),(11,2,'▃',Some(215),Some(167)),
    (12,2,'▘',Some(179),Some(238)),(13,2,'▌',Some(246),Some(238)),
    (0,3,'▗',Some(246),Some(236)),(1,3,'▗',Some(179),Some(239)),(2,3,'▃',Some(180),Some(215)),
    (3,3,'▗',Some(23),Some(108)),(4,3,'▔',Some(116),Some(238)),(5,3,'▂',Some(23),Some(109)),
    (6,3,'▍',Some(151),Some(215)),(7,3,'▕',Some(151),Some(215)),(8,3,'▞',Some(66),Some(109)),
    (9,3,'▋',Some(66),Some(73)),(10,3,'▎',Some(66),Some(72)),(11,3,'▏',Some(151),Some(179)),
    (12,3,'▞',Some(138),Some(236)),(13,3,'▖',Some(247),Some(233)),
    (0,4,'▖',Some(235),Some(244)),(1,4,'▍',Some(239),Some(137)),(2,4,'▘',Some(179),Some(173)),
    (3,4,'▔',Some(109),Some(173)),(4,4,'▆',Some(179),Some(239)),(5,4,'▆',Some(179),Some(66)),
    (6,4,'▔',Some(179),Some(173)),(7,4,'▔',Some(179),Some(173)),(8,4,'▔',Some(66),Some(179)),
    (9,4,'▆',Some(179),Some(235)),(10,4,'▔',Some(66),Some(173)),(11,4,'▕',Some(137),Some(173)),
    (12,4,'▋',Some(137),Some(237)),(13,4,'▄',Some(247),Some(59)),
    (0,5,'▊',Some(232),Some(145)),(1,5,'▍',Some(244),Some(237)),(2,5,'▝',Some(173),Some(239)),
    (3,5,'▃',Some(237),Some(179)),(4,5,'▂',Some(95),Some(179)),(5,5,'▁',Some(172),Some(215)),
    (6,5,'▂',Some(172),Some(215)),(7,5,'▂',Some(172),Some(215)),(8,5,'▂',Some(172),Some(215)),
    (9,5,'▂',Some(95),Some(180)),(10,5,'▃',Some(236),Some(179)),(11,5,'▘',Some(173),Some(8)),
    (12,5,'▝',Some(243),Some(237)),(13,5,'▌',Some(102),Some(234)),
    (0,6,'▕',Some(240),Some(233)),(1,6,'▎',Some(244),Some(59)),(2,6,'▝',Some(53),Some(60)),
    (3,6,'▘',Some(53),Some(60)),(4,6,'▔',Some(236),Some(54)),(5,6,'▄',Some(8),Some(239)),
    (6,6,'▆',Some(239),Some(130)),(7,6,'▅',Some(239),Some(130)),(8,6,'▗',Some(236),Some(240)),
    (9,6,'▔',Some(234),Some(54)),(10,6,'▕',Some(60),Some(54)),(11,6,'▘',Some(238),Some(60)),
    (12,6,'▝',Some(246),Some(237)),(13,6,'▖',Some(247),Some(235)),
];

pub struct IconWidget;

impl Widget for IconWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for &(col, row, ch, fg, bg) in ICON_CELLS {
            let x = area.x + col;
            let y = area.y + row;
            if x >= area.x + area.width || y >= area.y + area.height {
                continue;
            }
            let cell = &mut buf[(x, y)];
            cell.set_char(ch);
            let mut style = Style::default();
            if let Some(f) = fg {
                style = style.fg(Color::Indexed(f));
            }
            if let Some(b) = bg {
                style = style.bg(Color::Indexed(b));
            }
            cell.set_style(style);
        }
    }
}
