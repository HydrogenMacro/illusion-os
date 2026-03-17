use bytemuck::cast_slice;
use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;
use heapless::{String, Vec};

use crate::{display::{color::RGB565, display_layer::DisplayLayer}, flash_storage::FLASH_STORAGE};

pub struct FontData {
    pub offset: u32,
    pub char_width: u16,
    pub char_height: u16,
}

pub enum Font {
    _0xProto80,
    _0xProto40,
}

impl Font {
    pub const fn data(&self) -> FontData {
        match self {
            Font::_0xProto80 => FontData {
                char_width: 51,
                char_height: 127,
                offset: 0x210000,
            },
            Font::_0xProto40 => FontData {
                char_width: 23,
                char_height: 56,
                offset: 0x210000 + 44 * 110 * 64,
            },
        }
    }
    // data_buf should have len of >= 100
    pub fn get_char_data(&self, character: TextChar, data_buf: &mut [u8], offset_y: u16) {
        assert!(offset_y < self.data().char_height);
        assert!(data_buf.len() >= 100);

        let char_bytes = (self.data().char_width * self.data().char_height) as u32 * 2;
        
        let mut flash_storage =  FLASH_STORAGE.access();
        flash_storage.read(self.data().offset + char_bytes * character as u32 + self.data().char_width as u32 * 2 * offset_y as u32, &mut data_buf[0..2 * self.data().char_width as usize]).unwrap();
    }
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum TextChar {
    LwrA = 0,
    LwrB,
    LwrC,
    LwrD,
    LwrE,
    LwrF,
    LwrG,
    LwrH,
    LwrI,
    LwrJ,
    LwrK,
    LwrL,
    LwrM,
    LwrN,
    LwrO,
    LwrP,
    LwrQ,
    LwrR,
    LwrS,
    LwrT,
    LwrU,
    LwrV,
    LwrW,
    LwrX,
    LwrY,
    LwrZ,
    UprA,
    UprB,
    UprC,
    UprD,
    UprE,
    UprF,
    UprG,
    UprH,
    UprI,
    UprJ,
    UprK,
    UprL,
    UprM,
    UprN,
    UprO,
    UprP,
    UprQ,
    UprR,
    UprS,
    UprT,
    UprU,
    UprV,
    UprW,
    UprX,
    UprY,
    UprZ,
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Period,
    Colon = 63
}
pub struct Text {
    x: u16,
    y: u16,
    anchor: Anchor,
    font: Font,
    color: RGB565,
    text: Vec<TextChar, 20>,
}
impl Text {
    pub fn new(x: u16,
    y: u16,
    anchor: Anchor,
    font: Font,
    color: RGB565,
    text: Vec<TextChar, 20>,) -> Self {
        Text { x, y, anchor, font, color, text }
    }
    pub fn width(&self) -> u16 {
        return self.font.data().char_width * self.text.len() as u16;
    }
    pub fn height(&self) -> u16 {
        return self.font.data().char_height;
    }
}
pub enum Anchor {
    TopLeft,
    Center
}
impl DisplayLayer for Text {
    fn draw(&self, line_buf: &mut [super::color::RGB565], y: u16) {
        let mut char_data_buf = [0u8; 410];
        let char_width = self.font.data().char_width as usize;
        if y >= self.y && y < self.y + self.font.data().char_height {
            for &text_char in &self.text {
                self.font.get_char_data(text_char, &mut char_data_buf, y - self.y);
                line_buf[self.x as usize..self.x as usize + char_width].copy_from_slice(cast_slice(&char_data_buf[0..char_width * 2]));
            }
        }
    }
}
