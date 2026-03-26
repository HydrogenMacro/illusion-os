use ambassador::{Delegate, delegatable_trait};

pub mod text;
pub mod shape;


use crate::display::{color::RGB565, objects::text::Text};

#[delegatable_trait]
pub trait DisplayLayer {
    fn draw(&self, line_buf: &mut [RGB565], line_buf_y: u16);
}

#[derive(Delegate)]
#[delegate(DisplayLayer)]
pub enum DisplayObject {
    Text(Text),
}

