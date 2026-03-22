#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod display;
pub mod drivers;
pub mod flash_storage;
pub mod wallpaper;

use alloc::rc::Rc;
use bt_hci::controller::ExternalController;
use bytemuck::cast_slice_mut;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::*;
use esp_hal::gpio::*;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::BT;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{
    Blocking,
    clock::CpuClock,
    spi::master::{Config as SpiConfig, *},
};
use esp_radio::ble::Config as BleConfig;
use esp_radio::ble::controller::BleConnector;
use heapless::Vec;
use log::{error, info, warn};
use trouble_host::{Address, prelude::*};

use crate::display::color::RGB565;
use crate::display::display_layer::DisplayLayer;
use crate::display::drivers::co5300::CO5300;
use crate::display::text::{Anchor, Text, TextChar, TextString};
use crate::drivers::ble::WatchGATTServer;
use crate::drivers::i2c_bus::I2cBus;
use crate::drivers::pcf85063a::PCF85063A;
use crate::flash_storage::FLASH_STORAGE;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_storage::ReadStorage;
//use trouble_host::prelude::*;
extern crate alloc;

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

    let mut flash_storage = esp_storage::FlashStorage::new(peripherals.FLASH);

    FLASH_STORAGE.set(flash_storage);

    let btn1 = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Down),
    );

    let i2c = I2cBus::new(
        I2c::new(peripherals.I2C0, I2cConfig::default())
            .unwrap()
            .with_scl(peripherals.GPIO7)
            .with_sda(peripherals.GPIO8),
    );
    let mut rtc = Rc::new(RefCell::new(PCF85063A::new(i2c.clone())));

    spawner
        .spawn(run_ble(peripherals.BT, i2c.clone(), rtc.clone()))
        .unwrap();

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

    let mut time_text = Text::new(
        CO5300::WIDTH / 2,
        CO5300::HEIGHT / 2,
        Anchor::Center,
        display::text::Font::_0xProto120,
        |bg_color| bg_color.overlayed_with(RGB565::BLACK, 150),
    );

    loop {
        let rtc = rtc.borrow();
        let (time_hour_tens, time_hour_ones) = rtc.get_hour();
        let (time_min_tens, time_min_ones) = rtc.get_minute();
        let (time_sec_tens, time_sec_ones) = rtc.get_second();

        time_text.content.clear();
        time_text.content.push_bytestr(&[time_hour_tens + b'0']);
        time_text.content.push_bytestr(&[time_hour_ones + b'0']);
        time_text.content.push_bytestr(b":");
        time_text.content.push_bytestr(&[time_min_tens + b'0']);
        time_text.content.push_bytestr(&[time_min_ones + b'0']);

        //time_text.content.push_bytestr(&[time_secs_ones + b'0']);
        let draw_start_time = Instant::now();
        let mut draw_row_buf = [RGB565::BLACK; 410 * 2];
        for draw_cursor_y in (0..CO5300::HEIGHT).step_by(2) {
            FLASH_STORAGE
                .access()
                .read(
                    0x110000 + 410 * 2 * draw_cursor_y as u32,
                    cast_slice_mut(&mut draw_row_buf),
                )
                .unwrap();
            time_text.draw(&mut draw_row_buf[0..410], draw_cursor_y);
            time_text.draw(&mut draw_row_buf[410..410 * 2], draw_cursor_y + 1);
            //co5300.draw_pixels_with(0, draw_cursor_y, CO5300::WIDTH, 2, |x, y| draw_row_buf[((y - draw_cursor_y) * CO5300::WIDTH) as usize + x as usize]);
            co5300.draw_buf(0, draw_cursor_y, CO5300::WIDTH, 2, &draw_row_buf);
        }
        /*
        info!(
            "a {:?}",
            Instant::now().duration_since(draw_start_time).as_millis()
        );
        */
        Timer::after_millis(20).await;
    }
}

#[embassy_executor::task]
async fn run_ble(bt: BT<'static>, i2c: I2cBus, rtc: Rc<RefCell<PCF85063A>>) {
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    /*
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");*/
    let transport = BleConnector::new(&radio_init, bt, BleConfig::default()).unwrap();
    let ble_controller = ExternalController::<_, 1>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, 1, 2> = HostResources::new();

    let address = Address::random([0x44; 6]);

    let ble_stack = trouble_host::new(ble_controller, &mut resources).set_random_address(address);

    let trouble_host::Host {
        mut peripheral,
        mut runner,
        ..
    } = ble_stack.build();

    let gatt_server = WatchGATTServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "Omicron",
        appearance: &appearance::watch::SMARTWATCH,
    }))
    .unwrap();

    let mut battery_pct = [0; 1];
    i2c.try_access()
        .map(|mut i2c| i2c.write_read(0x34, &[0xA4], &mut battery_pct).unwrap())
        .unwrap();
    gatt_server
        .battery_service
        .level
        .set(&gatt_server, &battery_pct[0])
        .unwrap();
    let ble_advertisement_params = AdvertisementParameters {
        ..Default::default()
    };

    let mut advertiser_data = [0; 64];
    let _ = join(runner.run(), async {
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[
                    service::BATTERY.to_le_bytes(),
                    service::CURRENT_TIME.to_le_bytes(),
                ]),
                AdStructure::CompleteLocalName(b"Omicron"),
            ],
            &mut advertiser_data[..],
        )
        .unwrap();

        let advertiser = peripheral
            .advertise(
                &ble_advertisement_params,
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertiser_data[0..len],
                    scan_data: &[],
                },
            )
            .await
            .unwrap();

        let conn: GattConnection<_> = advertiser
            .accept()
            .await
            .unwrap()
            .with_attribute_server(&gatt_server)
            .unwrap();

        loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    info!("disconnected: {:?}", reason);
                    break;
                }
                GattConnectionEvent::Gatt { event } => {
                    match &event {
                        GattEvent::Read(event) => {
                            if event.handle() == gatt_server.battery_service.level.handle {
                                info!("battery read");
                            }
                            if event.handle() == gatt_server.time_query_service.current_time.handle
                            {
                                info!("current time read");
                            }
                        }
                        GattEvent::Write(write_event) => {
                            if write_event.handle() == gatt_server.battery_service.level.handle {
                                info!("battery wrote");
                            }
                            if write_event.handle() == gatt_server.time_query_service.current_time.handle
                            {
                                Timer::after_millis(500).await;
                                let val = gatt_server
                                    .get(&gatt_server.time_query_service.current_time)
                                    .unwrap();
                                info!("current time wrote {:?}", val);
                                if val != 0 {
                                    rtc.borrow().set_from_unix_epoch(val);
                                }
                            }
                        }
                        _ => {}
                    }
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => warn!("accept error: {e:?}"),
                    }
                }
                _ => {}
            }
        }
    })
    .await;
}
