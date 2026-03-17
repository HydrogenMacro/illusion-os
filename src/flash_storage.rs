use core::{cell::UnsafeCell, mem::MaybeUninit, ops::{Deref, DerefMut}, sync::atomic::{AtomicBool, AtomicU32, Ordering}};

use derive_more::{Deref, DerefMut};

pub struct FlashStorage(UnsafeCell<Option<esp_storage::FlashStorage<'static>>>);

unsafe impl Sync for FlashStorage {}
impl FlashStorage {
    pub fn set(&self, flash_storage: esp_storage::FlashStorage<'static>) {
        if FLASH_STORAGE_ACCESS_ACTIVE.load(Ordering::SeqCst) {
            panic!("flash access active when setting");
        }
        FLASH_STORAGE_ACCESS_ACTIVE.store(true, Ordering::SeqCst);
        unsafe { *self.0.get() = Some(flash_storage); }
        FLASH_STORAGE_ACCESS_ACTIVE.store(false, Ordering::SeqCst);
    }
    pub fn access(&self) -> FlashStorageGuard<'_> {
        if !FLASH_STORAGE_ACCESS_ACTIVE.load(Ordering::SeqCst) {
            unsafe { FlashStorageGuard::new((*self.0.get()).as_mut().unwrap()) }
        } else {
            panic!("access already active");
        }
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
        FLASH_STORAGE_ACCESS_ACTIVE.store(false,Ordering::SeqCst);
    }
}
static FLASH_STORAGE_ACCESS_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static FLASH_STORAGE: FlashStorage = FlashStorage(UnsafeCell::new(None));