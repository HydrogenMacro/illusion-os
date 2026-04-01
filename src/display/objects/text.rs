use alloc::boxed::Box;
use bytemuck::{cast_slice, cast_slice_mut};
use derive_more::{Debug, Deref, DerefMut};
use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;
use heapless::{String, Vec};
use log::*;
use num_enum::{FromPrimitive, IntoPrimitive};

use crate::{
    display::{color::RGB565, objects::DisplayLayer},
    flash_storage::FLASH_STORAGE,
};

pub struct FontData {
    pub offset: u32,
    pub char_width: u16,
    pub char_height: u16,
}

pub enum Font {
    Font1,
    Font2,
    Font3
}

impl Font {
    pub const fn data(&self) -> FontData {
        match self {
            Font::Font1 => FontData {
                char_width: 73,
                char_height: 144,
                offset: 0x210000,
            },
            Font::Font2 => FontData {
                char_width: 76,
                char_height: 189,
                offset: 0x260000,
            },
            Font::Font3 => FontData { offset: 0x310000, char_width: 65, char_height: 164 }
        }
    }
    // data_buf should have len of >= 100
    pub fn get_char_data(&self, character: TextChar, data_buf: &mut [u8], offset_y: u16) {
        assert!(offset_y < self.data().char_height);
        assert!(data_buf.len() >= 100);
        let char_bytes = (self.data().char_width * self.data().char_height) as u32;

        let mut flash_storage = FLASH_STORAGE.access();
        flash_storage
            .read(
                self.data().offset
                    + char_bytes * character as u32
                    + self.data().char_width as u32 * offset_y as u32,
                &mut data_buf[0..self.data().char_width as usize],
            )
            .unwrap();
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
#[derive(FromPrimitive, IntoPrimitive)]
pub enum TextChar {
    #[default]
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
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Period,
    Colon = 63,
}

pub struct Text {
    anchor_x: u16,
    anchor_y: u16,
    anchor: Anchor,
    font: Font,
    color: fn(RGB565) -> RGB565,
    pub content: TextString,
}
impl Text {
    pub fn new(
        anchor_x: u16,
        anchor_y: u16,
        anchor: Anchor,
        font: Font,
        color: fn(RGB565) -> RGB565,
    ) -> Self {
        Text {
            anchor_x,
            anchor_y,
            anchor,
            font,
            color,
            content: TextString::new(),
        }
    }
    pub fn position(&self) -> (u16, u16) {
        match self.anchor {
            Anchor::Center => (
                self.anchor_x.saturating_sub(self.width() / 2),
                self.anchor_y.saturating_sub(self.height() / 2),
            ),
            Anchor::TopLeft => (self.anchor_x, self.anchor_y),
        }
    }
    pub fn width(&self) -> u16 {
        return self.font.data().char_width * self.content.len() as u16;
    }
    pub fn height(&self) -> u16 {
        return self.font.data().char_height;
    }
}
#[derive(Debug, Deref, DerefMut, Default)]
pub struct TextString(Vec<TextChar, { TextString::MAX_LEN }>);

impl TextString {
    pub const MAX_LEN: usize = 20;
    pub fn new() -> Self {
        TextString::default()
    }
    pub fn push_bytestr(&mut self, bytestr: &[u8]) -> &mut Self {
        debug_assert!(bytestr.len() + self.len() <= 20);
        for i in 0..bytestr.len() {
            let text_char = match bytestr[i] {
                c @ b'a'..=b'z' => TextChar::from_primitive((c - b'a') + TextChar::LwrA as u8),
                c @ b'A'..=b'Z' => TextChar::from_primitive((c - b'A') + TextChar::UprA as u8),
                c @ b'0'..=b'9' => TextChar::from_primitive((c - b'0') + TextChar::Num0 as u8),
                b'.' => TextChar::Period,
                b':' => TextChar::Colon,
                invalid_char => panic!("invalid char '{invalid_char}'"),
            };
            self.push(text_char);
        }
        self
    }
}
pub enum Anchor {
    TopLeft,
    Center,
}
impl DisplayLayer for Text {
    fn draw(&self, line_buf: &mut [RGB565], line_buf_y: u16) {
        let mut char_data_buf = [0; 410];
        let char_width = self.font.data().char_width as usize;
        let (text_x, text_y) = self.position();
        if line_buf_y >= text_y && line_buf_y < text_y + self.height() {
            for (i, &text_char) in self.content.iter().enumerate() {
                let char_x = text_x as usize + char_width * i;
                if char_x >= 410 {
                    break;
                }
                self.font
                    .get_char_data(text_char, &mut char_data_buf, line_buf_y - text_y);

                for cursor_char_x in 0..char_width {
                    let cursor_x = char_x + cursor_char_x;
                    if cursor_x >= 410 {
                        break;
                    }
                    if char_data_buf[cursor_char_x] == 0 {
                    } else {
                        line_buf[cursor_x] = line_buf[cursor_x].overlayed_with(
                            (self.color)(line_buf[cursor_x]),
                            255 - char_data_buf[cursor_char_x],
                        );
                    }
                }
            }
        }
    }
}
