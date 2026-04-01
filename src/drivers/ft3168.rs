use core::cell::RefCell;

use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Level, Output, OutputConfig, Pull},
    i2c,
};
use log::{error, info};

use crate::drivers::i2c_bus::I2cBus;

pub struct FT3168 {
    i2c: I2cBus,
    touch_points: [Option<TouchPoint>; 2],
    reset_pin: RefCell<Output<'static>>,
    int_pin: Output<'static>,
}
impl FT3168 {
    pub const EVENT_FLAG_PRESS_START: u8 = 0b00;
    pub const EVENT_FLAG_PRESS_END: u8 = 0b01;
    pub const EVENT_FLAG_IS_PRESSING: u8 = 0b10;
    pub const EVENT_FLAG_NO_EVENT: u8 = 0b11;
    pub fn new(i2c: I2cBus, reset_pin: AnyPin<'static>, int_pin: AnyPin<'static>) -> Self {
        let reset_pin = Output::new(
            reset_pin,
            Level::High,
            OutputConfig::default().with_pull(Pull::Up),
        );
        let int_pin = Output::new(
            int_pin,
            Level::High,
            OutputConfig::default().with_pull(Pull::Up),
        );

        let mut driver = FT3168 {
            i2c,
            touch_points: [const { None }; 2],
            reset_pin: RefCell::new(reset_pin),
            int_pin,
        };

        driver.reset();
        return driver;
    }
    pub fn reset(&self) {
        self.reset_pin.borrow_mut().set_low();
        Delay::new().delay_millis(50);
        self.reset_pin.borrow_mut().set_high();
    }
    pub fn get_touch_point(&self) -> Option<(u16, u16)> {
        let mut i2c = self.i2c.try_access().unwrap();
        let mut buf = [0; 1];

        if let Err(e) = i2c.write_read(0x38, &[0x02], &mut buf) {
            error!("i2c err: {e}");
            self.reset();
        }
        if buf[0] == 0 {
            return None;
        }

        let mut touch1_x = 0u16;
        if let Err(e) = i2c.write_read(0x38, &[0x03], &mut buf) {
            error!("i2c err: {e}");
            self.reset();
        }
        touch1_x |= ((buf[0] & 0b1111) as u16) << 8;
        if let Err(e) = i2c.write_read(0x38, &[0x04], &mut buf) {
            error!("i2c err: {e}");
        }
        touch1_x |= buf[0] as u16;

        let mut touch1_y = 0u16;
        if let Err(e) = i2c.write_read(0x38, &[0x05], &mut buf) {
            error!("i2c err: {e}");
        };
        touch1_y |= ((buf[0] & 0b1111) as u16) << 8;
        if let Err(e) = i2c.write_read(0x38, &[0x06], &mut buf) {
            error!("i2c err: {e}");
        };
        touch1_y |= buf[0] as u16;
        return Some((touch1_x, touch1_y));
    }
}

pub struct TouchPoint {
    pub touch_id: u8,
    pub x: u16,
    pub y: u16,
    pub weight: u8,
}
