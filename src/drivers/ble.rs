use esp_hal::peripherals::BT;
use trouble_host::prelude::*;

pub struct BLEManager {}

impl BLEManager {
    pub fn new(bt: BT<'static>) -> Self {
        BLEManager {}
    }
}

// GATT Server definition
#[gatt_server]
pub struct WatchGATTServer {
    pub battery_service: BatteryService,
    pub time_query_service: TimeQueryService,
}

#[gatt_service(uuid = service::BATTERY)]
pub struct BatteryService {
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "battery_level", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify)]
    pub level: u8,
}

#[gatt_service(uuid = "285771a8-3a8a-414d-9e9f-fc8d181a4878")]
pub struct TimeQueryService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "unix_epoch", read, value = "Current Unix Epoch Time")]
    #[characteristic(uuid = "3853b93c-f4ee-4bdf-8083-31ca09862a33", write, notify)]
    pub current_time: i64,
}
