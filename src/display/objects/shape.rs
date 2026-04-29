use core::{
    f32::consts::{FRAC_1_SQRT_2, SQRT_2},
    prelude::v1,
};

use crate::{
    display::{color::RGB565, objects::Drawable},
    utils::f32::{distance_squared_between, hypot_squared_of, square_of},
};

pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub rounding: u16,
    pub color: RGB565,
    pub opacity: u8,
}
impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16, color: RGB565, opacity: u8) -> Self {
        Rect {
            color,
            height,
            rounding: 0,
            width,
            x,
            y,
            opacity,
        }
    }
    pub fn new_rounded(
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        rounding: u16,
        color: RGB565,
        opacity: u8,
    ) -> Self {
        Rect {
            color,
            height,
            rounding,
            width,
            x,
            y,
            opacity,
        }
    }
}
impl Drawable for Rect {
    fn draw(&self, line_buf: &mut [RGB565; 410], line_buf_y: u16) {
        if line_buf_y < self.y || line_buf_y > self.y + self.height - 1 {
            return;
        }
        if self.rounding != 0 {
            for x in self.x..(self.x + self.width) {
                if x >= self.x && x < self.x + self.width {
                    if (x < self.x + self.rounding || x > self.x + self.width - self.rounding)
                        && (line_buf_y < self.y + self.rounding
                            || line_buf_y > self.y + self.height - self.rounding)
                    {
                        let pos = (x as f32, line_buf_y as f32);
                        let rect_rounding_squared = square_of(self.rounding as f32);
                        for circle_center in [
                            (
                                (self.x + self.rounding) as f32,
                                (self.y + self.rounding) as f32,
                            ),
                            (
                                (self.x + self.width - self.rounding) as f32,
                                (self.y + self.rounding) as f32,
                            ),
                            (
                                (self.x + self.width - self.rounding) as f32,
                                (self.y + self.height - self.rounding) as f32,
                            ),
                            (
                                (self.x + self.rounding) as f32,
                                (self.y + self.height - self.rounding) as f32,
                            ),
                        ] {
                            if distance_squared_between(circle_center, pos) < rect_rounding_squared
                            {
                                line_buf[x as usize] =
                                    line_buf[x as usize].overlayed_with(self.color, self.opacity);
                            }
                        }
                    } else {
                        line_buf[x as usize] =
                            line_buf[x as usize].overlayed_with(self.color, self.opacity);
                    }
                }
            }
        } else {
            for x in self.x..(self.x + self.width) {
                line_buf[x as usize] =
                    line_buf[x as usize].overlayed_with(self.color, self.opacity);
            }
        }
    }
}
