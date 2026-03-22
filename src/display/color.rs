use bytemuck::{AnyBitPattern, Pod, Zeroable};
use derive_more::{Debug, Deref, DerefMut};

#[derive(Clone, Copy, Deref, DerefMut, Debug)]
#[repr(transparent)]
/// RGB565 color, represented by a big-endian u16
pub struct RGB565(pub u16);
unsafe impl Pod for RGB565 {}
unsafe impl Zeroable for RGB565 {}

impl RGB565 {
    pub const BLACK: Self = RGB565::new(0, 0, 0);
    pub const WHITE: Self = RGB565::new(31, 63, 31);
    pub const RED: Self = RGB565::new(31, 0, 0);
    pub const GREEN: Self = RGB565::new(0, 63, 0);
    pub const BLUE: Self = RGB565::new(0, 0, 31);

    /// r and b should be 5 bits long (from 0 to 31), g should be 6 bits long (from 0 to 63)
    pub const fn new(r: u16, g: u16, b: u16) -> Self {
        return RGB565(((r << 11) | (g << 5) | b).swap_bytes());
    }
    /// returns a value from 0-31
    pub const fn red(&self) -> u16 {
        (self.0.swap_bytes() >> 11) & 0b11111
    }
    /// returns a value from 0-63
    pub const fn green(&self) -> u16 {
        (self.0.swap_bytes() >> 5) & 0b111111
    }
    /// returns a value from 0-31
    pub const fn blue(&self) -> u16 {
        (self.0.swap_bytes() >> 0) & 0b11111
    }
    /// returns the alpha composited color
    /// 0 <= alpha <= 255
    pub fn overlayed_with(&self, overlayed_color: RGB565, alpha: u8) -> RGB565 {
        let alpha = alpha as u16;
        RGB565::new(
            alpha_blend(self.red(), overlayed_color.red(), alpha),
            alpha_blend(self.green(), overlayed_color.green(), alpha),
            alpha_blend(self.blue(), overlayed_color.blue(), alpha),
        )
    }
}

/// for blending individual channels
pub const fn alpha_blend(c1: u16, c2: u16, alpha: u16) -> u16 {
    debug_assert!(alpha <= 255, "alpha must be in the range 0-255");
    ((c1 * alpha) >> 8) + (((255 - alpha) * c2) >> 8)
}
