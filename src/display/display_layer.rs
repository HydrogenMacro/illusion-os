use crate::display::color::RGB565;

pub trait DisplayLayer {
    fn draw(&self, line_buf: &mut [RGB565], y: u16);
}
