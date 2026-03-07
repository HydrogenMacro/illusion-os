use derive_more::{Debug, Deref, DerefMut};

#[derive(Clone, Copy, Deref, DerefMut)]
pub struct RGB565(pub u16);
impl RGB565 {
    /// r and b should be 5 bits long (from 0 to 31), g should be 6 bits long (from 0 to 63)
    pub const fn new(r: u16, g: u16, b: u16) -> Self {
        return RGB565(((r << 11) | (g << 5) | b).to_be());
    }
    pub const BLACK: Self = RGB565::new(0, 0, 0);
    pub const WHITE: Self = RGB565::new(31, 63, 31);
    pub const RED: Self = RGB565::new(31, 0, 0);
    pub const GREEN: Self = RGB565::new(0, 63, 0);
    pub const BLUE: Self = RGB565::new(0, 0, 31);
}
