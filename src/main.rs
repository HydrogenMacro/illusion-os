#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
pub mod co5300;
pub mod co5300_commands;

use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
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
use log::{error, info};
//use trouble_host::prelude::*;
use crate::co5300_commands::*;
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

    let mut lcd_reset = Output::new(
        peripherals.GPIO11,
        Level::High,
        OutputConfig::default(),
    );
    lcd_reset.set_low();
    lcd_reset.set_high();

    let mut spi_cs = Output::new(
        peripherals.GPIO15,
        Level::High,
        OutputConfig::default(),
    );
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(4096);

    let dma_rx_buf = esp_hal::dma::DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();

    let dma_tx_buf = esp_hal::dma::DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default().with_frequency(Rate::from_mhz(80)),
    )
    .unwrap()
    .with_cs(peripherals.GPIO5)
    .with_sck(peripherals.GPIO0)
    .with_sio0(peripherals.GPIO1)
    .with_sio1(peripherals.GPIO2)
    .with_sio2(peripherals.GPIO3)
    .with_sio3(peripherals.GPIO4)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf);
    lcd_reset.set_low();
    lcd_reset.set_high();

    spi_write_cmd(&mut spi, CO5300_C_SLPOUT, &mut spi_cs);

    spi_write_c8d8(&mut spi, 0xFE, 0x00, &mut spi_cs);
    spi_write_c8d8(&mut spi, CO5300_W_SPIMODECTL, 0x80, &mut spi_cs);
    spi_write_c8d8(&mut spi, CO5300_W_PIXFMT, 0x55, &mut spi_cs);
    spi_write_c8d8(&mut spi, CO5300_W_WCTRLD1, 0x20, &mut spi_cs);
    spi_write_c8d8(&mut spi, CO5300_W_WDBRIGHTNESSVALHBM, 0xFF, &mut spi_cs);
    spi_write_cmd(&mut spi, CO5300_C_DISPON, &mut spi_cs);
    spi_write_c8d8(&mut spi, CO5300_W_WDBRIGHTNESSVALNOR, 0xA0, &mut spi_cs);

    spi_write_c8d8(
        &mut spi,
        CO5300_W_MADCTL,
        CO5300_MADCTL_COLOR_ORDER,
        &mut spi_cs,
    );
    spi_write_cmd(&mut spi, CO5300_C_INVOFF, &mut spi_cs);

    spi_write_c8d16d16(&mut spi, CO5300_W_CASET, 50 + 22, 70 - 1 + 22, &mut spi_cs);
    spi_write_c8d16d16(&mut spi, CO5300_W_PASET, 50, 70 - 1, &mut spi_cs);
    spi_write_cmd(&mut spi, CO5300_W_RAMWR, &mut spi_cs);

    spi_cs.set_low();
    spi.half_duplex_write(
        DataMode::Quad,
        Command::_8Bit(0x32, DataMode::Single),
        Address::_24Bit(0x003C00, DataMode::Single),
        0,
        &[0xCD; 20 * 20],
    )
    .unwrap();
    spi.half_duplex_write(
        DataMode::Quad,
        Command::None,
        Address::None,
        0,
        &[0xFF; 20 * 20],
    )
    .unwrap();
    spi_cs.set_high();

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
