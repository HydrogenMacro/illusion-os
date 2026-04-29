use ambassador::{Delegate, delegatable_trait};
use derive_more::Add;

pub mod shape;
pub mod text;

use crate::display::{
    color::RGB565,
    objects::{shape::Rect, text::Text},
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ZLevel {
    /// for the wallpaper
    Background(i8),
    /// for the main content in the watch, most things should be on this level
    Content(i8),
    /// for modals and popovers
    Modal(i8),
    /// for absolute items that must be at the top, like the input matrix and notifications
    Top(i8),
}

impl ZLevel {
    pub const BACKGROUND: i8 = -100;
    pub const ITEM: i8 = 0;
    pub const TEXT: i8 = 100;
}
#[delegatable_trait]
pub trait Drawable {
    fn draw(&self, line_buf: &mut [RGB565; 410], line_buf_y: u16);
}

#[derive(Delegate)]
#[delegate(Drawable)]
pub enum DrawableItem {
    Text(DisplayObject<Text>),
    Rect(DisplayObject<Rect>),
}

pub struct DisplayObject<T: Drawable> {
    pub object: T,
    pub z_level: ZLevel,
}
impl<T: Drawable> DisplayObject<T> {
    pub fn new(object: T, z_level: ZLevel) -> Self {
        DisplayObject { object, z_level }
    }
}
impl<T: Drawable> Drawable for DisplayObject<T> {
    fn draw(&self, line_buf: &mut [RGB565; 410], line_buf_y: u16) {
        self.object
            .draw(line_buf, line_buf_y);
    }
}
