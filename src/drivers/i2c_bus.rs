use core::{cell::RefCell, ops::DerefMut};

use alloc::rc::Rc;
use esp_hal::{Blocking, i2c::master::I2c};

#[derive(Debug)]
pub struct I2cBus(Rc<RefCell<I2c<'static, Blocking>>>);
impl I2cBus {
    pub fn new(i2c: I2c<'static, Blocking>) -> Self {
        I2cBus(Rc::new(RefCell::new(i2c)))
    }
    pub fn try_access(&self) -> Option<impl DerefMut<Target = I2c<'static, Blocking>> + use<'_>> {
        self.0.try_borrow_mut().ok()
    }
}
impl Clone for I2cBus {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}