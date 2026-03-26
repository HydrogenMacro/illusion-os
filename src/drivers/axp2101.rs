use crate::drivers::i2c_bus::I2cBus;

pub struct Axp2101 {
    i2c: I2cBus
}
impl Axp2101 {
    pub fn new(i2c: I2cBus) -> Self {
        Axp2101 { i2c }
    }
    // get battery percentage from 0-100
    pub fn get_battery_pct(&self) -> u8 {
        let mut battery_pct = [0; 1];
        self.i2c.try_access()
            .map(|mut i2c| i2c.write_read(0x34, &[0xA4], &mut battery_pct).unwrap())
            .unwrap();
        return battery_pct[0];
    }
}