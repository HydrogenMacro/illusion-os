use core::{cell::UnsafeCell, mem::MaybeUninit, ops::{Deref, DerefMut}, sync::atomic::{AtomicBool, AtomicU32, Ordering}};

use derive_more::{Deref, DerefMut};

pub struct FlashStorage(UnsafeCell<Option<esp_storage::FlashStorage<'static>>>);

unsafe impl Sync for FlashStorage {}
impl FlashStorage {
    fn is_busy(&self) -> bool {
        FLASH_STORAGE_ACCESS_ACTIVE.load(Ordering::SeqCst)
    }
    fn set_is_busy(&self, is_busy: bool) {
        FLASH_STORAGE_ACCESS_ACTIVE.store(is_busy, Ordering::SeqCst);
    }
    pub fn set(&self, flash_storage: esp_storage::FlashStorage<'static>) {
        if self.is_busy() {
            panic!("flash access active when setting");
        }
        self.set_is_busy(true);
        unsafe { *self.0.get() = Some(flash_storage); }
        self.set_is_busy(false);
    }
    pub fn access(&self) -> FlashStorageGuard<'_> {
        if self.is_busy() {
            panic!("access already active");
        }
        self.set_is_busy(true);
        unsafe { FlashStorageGuard::new((*self.0.get()).as_mut().unwrap()) }
    }
}

#[derive(Deref, DerefMut)]
pub struct FlashStorageGuard<'a>(&'a mut esp_storage::FlashStorage<'static>);

impl<'a> FlashStorageGuard<'a> {
    fn new(flash_storage_ref: &'a mut esp_storage::FlashStorage<'static>) -> Self {
        FlashStorageGuard(flash_storage_ref)
    }
}
impl Drop for FlashStorageGuard<'_> {
    fn drop(&mut self) {
        FLASH_STORAGE.set_is_busy(false);
    }
}
static FLASH_STORAGE_ACCESS_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static FLASH_STORAGE: FlashStorage = FlashStorage(UnsafeCell::new(None));