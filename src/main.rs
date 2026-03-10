#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod display;
pub mod wallpaper;

use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::*;
use esp_hal::gpio::*;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{
    Blocking,
    clock::CpuClock,
    spi::master::{Config as SpiConfig, *},
};
use esp_radio::ble::controller::BleConnector;
use esp_storage::FlashStorage;
use log::{error, info};
use embedded_storage::*;

use crate::display::co5300::CO5300;
use crate::display::color::RGB565;
//use trouble_host::prelude::*;
extern crate alloc;

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(&radio_init, peripherals.BT, Default::default()).unwrap();
    //let ble_controller = ExternalController::<_, 1>::new(transport);
    //let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
    //    HostResources::new();
    //let _stack = trouble_host::new(ble_controller, &mut resources);

    let mut flash_storage = FlashStorage::new(peripherals.FLASH);
    let mut partition_table_data = [0; PARTITION_TABLE_MAX_LEN];
    let partition_table = read_partition_table(&mut flash_storage, &mut partition_table_data).unwrap();
    let wallpaper_partition = partition_table.find_partition(PartitionType::Data(DataPartitionSubType::Undefined)).unwrap().unwrap();
    let mut wallpaper_partition = wallpaper_partition.as_embedded_storage(&mut flash_storage);
    
    let btn1 = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Down),
    );

    let mut i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_scl(peripherals.GPIO7)
        .with_sda(peripherals.GPIO8);

    let mut a = [0; 2];
    i2c.write_read(0x34, &[0xA4], &mut a).unwrap();

    info!("aaa {:?}", a);
    
    let mut co5300 = CO5300::new(
        peripherals.GPIO11,
        peripherals.SPI2,
        peripherals.DMA_CH0,
        peripherals.GPIO0,
        peripherals.GPIO1,
        peripherals.GPIO2,
        peripherals.GPIO3,
        peripherals.GPIO4,
        peripherals.GPIO5,
    );
    co5300.init().await;
    let mut cc = [0; 410 * 2];
    wallpaper_partition.read(0, &mut cc).unwrap();
    let mut current_y = 0u32;
    co5300.draw_pixels(0, 0, 410, 502, |px, py| {
        if current_y != py as u32 {
            current_y = py as u32;
            info!("reading {}", 410 * 2 * current_y);
            wallpaper_partition.read(410 * 2 * current_y as u32, &mut cc).unwrap();
        }
        let px_idx = px as usize * 2;
        //RGB565::new(0, 63, 31)
        RGB565(u16::from_le_bytes([cc[px_idx], cc[px_idx + 1]]))
    });
    
    info!("{}", 1);
    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }
}
fn spi_write_cmd(spi: &mut SpiDmaBus<Blocking>, cmd: u8, cs_pin: &mut Output) {
    cs_pin.set_low();
    spi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(0x02, DataMode::Single),
        Address::_24Bit((cmd as u32) << 8, DataMode::Single),
        0,
        &[],
    )
    .unwrap();
    cs_pin.set_high();
}
fn spi_write_c8d8(spi: &mut SpiDmaBus<Blocking>, cmd: u8, d: u8, cs_pin: &mut Output) {
    cs_pin.set_low();
    spi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(0x02, DataMode::Single),
        Address::_24Bit((cmd as u32) << 8, DataMode::Single),
        0,
        &[d],
    )
    .unwrap();
    cs_pin.set_high();
}
fn spi_write_c8d16d16(
    spi: &mut SpiDmaBus<Blocking>,
    cmd: u8,
    d1: u16,
    d2: u16,
    cs_pin: &mut Output,
) {
    cs_pin.set_low();
    spi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(0x02, DataMode::Single),
        Address::_24Bit((cmd as u32) << 8, DataMode::Single),
        0,
        &[(d1 >> 8) as u8, d1 as u8, (d2 >> 8) as u8, d2 as u8],
    )
    .unwrap();
    cs_pin.set_high();
}
pub fn send_cmd<const N: usize>(
    spi: &mut SpiDmaBus<Blocking>,
    cmd: u8,
    data: [u8; N],
    cs_pin: &mut Output,
) {
    cs_pin.set_low();
    spi.half_duplex_write(
        DataMode::Single,
        Command::_8Bit(0x02, DataMode::Single),
        Address::_24Bit((cmd as u32) << 8, DataMode::Single),
        0,
        &data,
    )
    .unwrap();
    cs_pin.set_high();
}
