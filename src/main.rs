#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
//#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![feature(iter_map_windows)]
#![feature(str_as_str)]

mod display;
pub mod drivers;
pub mod scenes;
pub mod flash_storage;
pub mod input_matrix;
pub mod utils;

use crate::display::color::RGB565;
use crate::display::drivers::co5300::CO5300;
use crate::display::objects::shape::Rect;
use crate::display::objects::text::{Anchor, Font, Text, TextChar, TextString};
use crate::drivers::axp2101::Axp2101;
use crate::drivers::ble::WatchGATTServer;
use crate::drivers::ft3168::FT3168;
use crate::drivers::i2c_bus::I2cBus;
use crate::drivers::pcf85063a::PCF85063A;
use crate::flash_storage::FLASH_STORAGE;
use crate::input_matrix::driver::{InputMatrixDriver, input_matrix_task};
use alloc::rc::Rc;
use bt_hci::controller::ExternalController;
use bytemuck::cast_slice_mut;
use core::cell::RefCell;
use display::objects::Drawable;
use embassy_executor::Spawner;
use embassy_futures::join::{join, join3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_storage::ReadStorage;
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::*;
use esp_hal::gpio::interconnect::PeripheralInput;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::{BT, LPWR};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{
    Blocking,
    clock::CpuClock,
    spi::master::{Config as SpiConfig, *},
};
use esp_hal::{gpio::*, system};
use esp_radio::ble::Config as BleConfig;
use esp_radio::ble::controller::BleConnector;
use heapless::Vec;
use log::{error, info, warn};
use trouble_host::{Address, prelude::*};
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
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let mut flash_storage = esp_storage::FlashStorage::new(peripherals.FLASH);

    FLASH_STORAGE.set(flash_storage);

    let i2c = I2cBus::new(
        I2c::new(
            peripherals.I2C0,
            I2cConfig::default().with_frequency(Rate::from_khz(10)),
        )
        .unwrap()
        .with_scl(peripherals.GPIO7)
        .with_sda(peripherals.GPIO8),
    );
    let mut rtc = Rc::new(RefCell::new(PCF85063A::new(i2c.clone())));
    let mut battery = Rc::new(RefCell::new(Axp2101::new(i2c.clone())));
    let touch_controller = Rc::new(RefCell::new(FT3168::new(
        i2c.clone(),
        peripherals.GPIO10.into(),
        peripherals.GPIO15.into(),
    )));
    let input_matrix_driver = Rc::new(RefCell::new(InputMatrixDriver::new()));

    spawner
        .spawn(input_matrix_task(touch_controller.clone(), input_matrix_driver.clone()))
        .unwrap();
    spawner
        .spawn(run_ble(peripherals.BT, battery.clone(), rtc.clone()))
        .unwrap();

    let mut co5300 = Rc::new(RefCell::new(CO5300::new(
        peripherals.GPIO11,
        peripherals.SPI2,
        peripherals.DMA_CH0,
        peripherals.GPIO0,
        peripherals.GPIO1,
        peripherals.GPIO2,
        peripherals.GPIO3,
        peripherals.GPIO4,
        peripherals.GPIO5,
    )));
    co5300.borrow_mut().init().await;

    spawner
        .spawn(boot_btn_task(
            peripherals.GPIO9.into(),
            peripherals.LPWR,
            co5300.clone(),
        ))
        .unwrap();

    let mut scene: Vec<display::objects::DrawableItem, 10> = Vec::new();
    let mut time_text = Text::new(
        CO5300::WIDTH / 2,
        CO5300::HEIGHT / 2,
        Anchor::Center,
        Font::Font3,
        |bg_color| bg_color.invert().overlayed_with(RGB565::WHITE, 70), // bg_color.overlayed_with(RGB565::BLACK, 150),
    );
    let mut rect = Rect::new_rounded(40, 40, 80, 80, 40, RGB565::BLUE, 170);
    loop {
        if co5300.borrow().display_off {
            Timer::after_millis(100).await;
            continue;
        }
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
        let mut draw_buf = [RGB565::BLACK; 410 * 2];
        for draw_cursor_y in (0..CO5300::HEIGHT).step_by(2) {
            FLASH_STORAGE
                .access()
                .read(
                    0x110000 + CO5300::WIDTH as u32 * 2 * draw_cursor_y as u32,
                    cast_slice_mut(&mut draw_buf),
                )
                .unwrap();
            for (i, line_buf) in draw_buf.as_chunks_mut::<{ CO5300::WIDTH as usize }>().0.iter_mut().enumerate() {
                time_text.draw(line_buf, draw_cursor_y + i as u16);
                rect.draw(line_buf, draw_cursor_y + i as u16);
            }
            //co5300.draw_pixels_with(0, draw_cursor_y, CO5300::WIDTH, 2, |x, y| draw_row_buf[((y - draw_cursor_y) * CO5300::WIDTH) as usize + x as usize]);
            
            for a in &scene {
                //a.draw(&mut draw_row_buf[..], draw_cursor_y, 2);
            }

            co5300
                .borrow_mut()
                .draw_buf(0, draw_cursor_y, CO5300::WIDTH, 2, &draw_buf);
        }

        /*
        info!(
            "frame time {:?}",
            Instant::now().duration_since(draw_start_time).as_millis()
        );
        */
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn boot_btn_task(
    btn_gpio: AnyPin<'static>,
    lpwr: LPWR<'static>,
    co5300: Rc<RefCell<CO5300>>,
) {
    let mut boot_btn = Input::new(btn_gpio, InputConfig::default().with_pull(Pull::Down));
    let mut rtc = Rtc::new(lpwr);
    let mut display_is_on = true;
    loop {
        boot_btn.wait_for_rising_edge().await;
        info!("boot btn hi");
        if display_is_on {
            co5300.borrow_mut().display_off();
        } else {
            co5300.borrow_mut().display_on();
        }
        display_is_on = !display_is_on;
        boot_btn.wait_for_falling_edge().await;
        info!("boot btn lo");
    }
}
#[embassy_executor::task]
async fn run_ble(bt: BT<'static>, battery: Rc<RefCell<Axp2101>>, rtc: Rc<RefCell<PCF85063A>>) {
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

    let ble_advertisement_params = AdvertisementParameters {
        ..Default::default()
    };

    let mut advertiser_data = [0; 64];
    let _ = join3(
        runner.run(),
        async {
            loop {
                gatt_server
                    .battery_service
                    .level
                    .set(&gatt_server, &battery.borrow_mut().get_battery_pct())
                    .unwrap();
                Timer::after_millis(5000).await;
            }
        },
        async {
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
            loop {
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
                                    if event.handle()
                                        == gatt_server.time_query_service.current_time.handle
                                    {
                                        info!("current time read");
                                    }
                                }
                                GattEvent::Write(write_event) => {
                                    if write_event.handle()
                                        == gatt_server.battery_service.level.handle
                                    {
                                        info!("battery wrote");
                                    }
                                    if write_event.handle()
                                        == gatt_server.time_query_service.current_time.handle
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
            }
        },
    )
    .await;
}
