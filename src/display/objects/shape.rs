use crate::display::{color::RGB565, objects::DisplayLayer};

pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub rounding: u16,
    pub color: fn(u16, u16) -> RGB565,
}
impl DisplayLayer for Rect {
    fn draw(&self,line_buf: &mut [RGB565],line_buf_y:u16) {
        todo!()
    }
}