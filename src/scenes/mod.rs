use crate::display::objects::{DisplayObject, DrawableItem};

pub mod watch_face;

pub trait Scene {
    fn redraw();
}
