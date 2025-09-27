#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    pub const fn align(&self, value: i32) -> i32 {
        value * (*self as i32) / 2
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum VerticalAlignment {
    Top,
    Middle,
    Bottom,
}

impl VerticalAlignment {
    pub const fn align(&self, value: i32) -> i32 {
        value * (*self as i32) / 2
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

impl Alignment {
    pub const CENTER: Alignment =
        Alignment::new(HorizontalAlignment::Center, VerticalAlignment::Middle);
    pub const LEFT: Alignment =
        Alignment::new(HorizontalAlignment::Left, VerticalAlignment::Middle);

    pub const fn new(horizontal: HorizontalAlignment, vertical: VerticalAlignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

impl From<(HorizontalAlignment, VerticalAlignment)> for Alignment {
    fn from((horizontal, vertical): (HorizontalAlignment, VerticalAlignment)) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}
