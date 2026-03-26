use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike};
use log::info;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

use crate::drivers::i2c_bus::I2cBus;

/// Driver for the PCF85063A RTC clock
/// This operates in 24hr mode
pub struct PCF85063A {
    i2c: I2cBus,
    timezone_utc_offset_hrs: i32
}
impl PCF85063A {
    pub const ADDRESS: u8 = 0x51;
    pub fn new(i2c: I2cBus) -> Self {
        let pcf = PCF85063A { i2c, timezone_utc_offset_hrs: -5 };
        pcf.update_config();
        pcf
    }
    fn update_config(&self) {
        self.i2c
            .try_access()
            .map(|mut i2c| i2c.write(PCF85063A::ADDRESS, &[0x00, 0b00000000]));
    }
    pub fn set_second(&self, second: u8) {
        debug_assert!((second < 60));
        let (tens, ones) = (second / 10, second % 10);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x04, ones | (tens << 4) | (1 << 7)])
                .unwrap();
        }
    }
    pub fn set_minute(&self, minute: u8) {
        debug_assert!(minute < 60);
        let (tens, ones) = ((minute / 10) & 0b1111, (minute % 10) & 0b1111);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x05, ones | (tens << 4)])
                .unwrap();
        }
    }
    pub fn set_hour(&self, hour: u8) {
        debug_assert!(hour < 23);
        let (tens, ones) = (hour / 10, hour % 10);
        info!("{tens} {ones}");
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x06, ones | (tens << 4)])
                .unwrap();
        }
    }
    pub fn set_day(&self, day: u8) {
        debug_assert!(1 <= day && day <= 31);
        let (tens, ones) = (day / 10, day % 10);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x07, ones | (tens << 4)])
                .unwrap();
        }
    }
    pub fn set_weekday(&self, weekday: u8) {
        debug_assert!(weekday < 7);
        let (tens, ones) = (weekday / 10, weekday % 10);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x08, ones | (tens << 4)])
                .unwrap();
        }
    }
    pub fn set_month(&self, month: u8) {
        debug_assert!(1 <= month && month < 12);
        let (tens, ones) = (month / 10, month % 10);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x09, ones | (tens << 4)]).unwrap();
        }
    }
    pub fn set_year(&self, year: u8) {
        debug_assert!(year < 99);
        let (tens, ones) = (year / 10, year % 10);
        if let Some(mut i2c) = self.i2c.try_access() {
            i2c.write(PCF85063A::ADDRESS, &[0x0A, ones | (tens << 4)]).unwrap();
        }
    }
    pub fn set_from_unix_epoch(&self, unix_epoch: i64) {
        let date_time: DateTime<FixedOffset> = DateTime::from_timestamp_secs(unix_epoch)
            .unwrap()
            // timezone is -1 cuz i cbb to deal with daylight savings
            .with_timezone(&TimeZone::from_offset(&FixedOffset::east_opt((self.timezone_utc_offset_hrs - 1) * 3600).unwrap()));
        self.set_second(date_time.second() as u8);
        self.set_minute(date_time.minute() as u8);
        self.set_hour(date_time.hour() as u8);
        self.set_day(date_time.day() as u8);
        self.set_month(date_time.month() as u8);
        self.set_year((date_time.year() - 2000) as u8);
        self.set_weekday(date_time.weekday().num_days_from_sunday() as u8);
    }
    /// (tens place, ones place)
    pub fn get_second(&self) -> (u8, u8) {
        let mut buf = [0; 1];
        self.i2c
            .try_access()
            .map(|mut i2c| i2c.write_read(PCF85063A::ADDRESS, &[0x04], &mut buf));
        (buf[0] >> 4 & 0b111, buf[0] & 0b1111)
    }

    /// (tens place, ones place)
    pub fn get_minute(&self) -> (u8, u8) {
        let mut buf = [0; 1];
        self.i2c
            .try_access()
            .map(|mut i2c| i2c.write_read(PCF85063A::ADDRESS, &[0x05], &mut buf));
        (buf[0] >> 4 & 0b111, buf[0] & 0b1111)
    }
    /// (tens place, ones place)
    pub fn get_hour(&self) -> (u8, u8) {
        let mut buf = [0; 1];
        self.i2c
            .try_access()
            .map(|mut i2c| i2c.write_read(PCF85063A::ADDRESS, &[0x06], &mut buf));
        (buf[0] >> 4 & 0b1, buf[0] & 0b1111)
    }
}

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
enum Weekday {
    Sunday = 0,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}
impl Weekday {
    pub fn to_full_name(&self) -> &'static str {
        match &self {
            Weekday::Sunday => "Sunday",
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
        }
    }
    pub fn to_short_name(&self) -> &'static str {
        match &self {
            Weekday::Sunday => "Sun",
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
        }
    }
}
