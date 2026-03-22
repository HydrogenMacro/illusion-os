use bytemuck::{cast, cast_ref, cast_slice};
use embassy_time::Timer;
use esp_hal::{
    Blocking,
    dma::{DmaChannel, DmaChannelFor},
    gpio::{interconnect::PeripheralOutput, *},
    spi::master::{Config as SpiConfig, Instance as SpiInstance, *},
    time::Rate,
};
use heapless::Vec;
use log::info;

use super::{super::color::RGB565, co5300_commands::*};

/// A CO5300 driver implementation
///
/// This is specifically hardcoded for the Waveshare ESP32-C6 2.06' AMOLED watch
pub struct CO5300 {
    pub spi: SpiDmaBus<'static, Blocking>,
    pub reset_pin: Output<'static>,
    pub cs_pin: Output<'static>,
    pub pixel_buf: Vec<u16, { Self::MAX_PIXELS_SENT_AT_ONCE as usize }>,
    
}
impl CO5300 {
    pub const MAX_PIXELS_SENT_AT_ONCE: u32 = 1024;
    pub const WIDTH: u16 = 410;
    pub const HEIGHT: u16 = 502;
    pub const COL_OFFSET: u16 = 22;
    pub const SPI_FREQUENCY_MHZ: u32 = 40;
    pub fn new(
        reset_pin: impl OutputPin + 'static,
        spi: impl SpiInstance + 'static,
        dma: impl DmaChannelFor<AnySpi<'static>>,
        sck: impl OutputPin + 'static,
        sio0: impl PeripheralOutput<'static>,
        sio1: impl PeripheralOutput<'static>,
        sio2: impl PeripheralOutput<'static>,
        sio3: impl PeripheralOutput<'static>,
        cs: impl OutputPin + 'static,
    ) -> Self {
        let mut reset_pin = Output::new(reset_pin, Level::High, OutputConfig::default());
        let mut cs_pin = Output::new(cs, Level::High, OutputConfig::default());
        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) =
            esp_hal::dma_buffers!(CO5300::MAX_PIXELS_SENT_AT_ONCE as usize * size_of::<u16>());

        let dma_rx_buf = esp_hal::dma::DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();

        let dma_tx_buf = esp_hal::dma::DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

        let mut spi = Spi::new(
            spi,
            SpiConfig::default().with_frequency(Rate::from_mhz(Self::SPI_FREQUENCY_MHZ)),
        )
        .unwrap()
        .with_sck(sck)
        .with_sio0(sio0)
        .with_sio1(sio1)
        .with_sio2(sio2)
        .with_sio3(sio3)
        .with_dma(dma)
        .with_buffers(dma_rx_buf, dma_tx_buf);

        CO5300 {
            spi,
            cs_pin,
            reset_pin,
            pixel_buf: Vec::new(),
        }
    }

    pub async fn reset(&mut self) {
        self.reset_pin.set_low();
        self.reset_pin.set_high();
    }
    /// The initialization function for the driver
    /// This must be called before anything can be drawn.
    pub async fn init(&mut self) {
        self.reset().await;

        self.send_cmd(CO5300_C_SLPOUT, []);

        self.send_cmd(0xFE, [0x00]);
        self.send_cmd(CO5300_W_SPIMODECTL, [0x80]);
        self.send_cmd(CO5300_W_PIXFMT, [0b01010101]);
        self.send_cmd(CO5300_W_WCTRLD1, [0x20]);
        self.send_cmd(CO5300_W_WDBRIGHTNESSVALHBM, [0xFF]);
        self.send_cmd(CO5300_C_DISPON, []);
        self.send_cmd(CO5300_W_WDBRIGHTNESSVALNOR, [0xA0]);
        self.send_cmd(CO5300_W_MADCTL, [CO5300_MADCTL_COLOR_ORDER]);
        self.send_cmd(CO5300_C_INVOFF, []);
    }
    pub fn send_cmd<const N: usize>(&mut self, cmd: u8, data: [u8; N]) {
        self.cs_pin.set_low();
        self.spi
            .half_duplex_write(
                DataMode::Single,
                Command::_8Bit(0x02, DataMode::Single),
                Address::_24Bit((cmd as u32) << 8, DataMode::Single),
                0,
                &data,
            )
            .unwrap();
        self.cs_pin.set_high();
    }

    /// Draws on the screen in the rectangle area.
    ///
    /// rect x, y, width, and height must all be even (co5300 restrictions)
    /// 
    /// For each pixel in the area (amount is), the pixel function is passed with the pixel's absolute screen x and y to produce a color.
    /// For example, to have a constant color, just pass in `|_, _| RGB565::new(...)`, and you can also use them to index a custom buffer
    pub fn draw_pixels_with(
        &mut self,
        rect_x: u16,
        rect_y: u16,
        rect_w: u16,
        rect_h: u16,
        mut pixel_fn: impl FnMut(u16, u16) -> RGB565,
    ) {
        debug_assert!(rect_x % 2 == 0);
        debug_assert!(rect_y % 2 == 0);
        debug_assert!(rect_w % 2 == 0);
        debug_assert!(rect_h % 2 == 0);

        let rect_end_x = rect_x + rect_w - 1;
        let rect_end_y = rect_y + rect_h - 1;
        let mut total_pixels_to_send = rect_w as u32 * rect_h as u32;
        let mut sent_pixels = 0;
        self.send_cmd(
            CO5300_W_CASET,
            cast::<_, [u8; 4]>([
                (rect_x + Self::COL_OFFSET).to_be(),
                (rect_end_x + Self::COL_OFFSET).to_be(),
            ]),
        );
        self.send_cmd(
            CO5300_W_PASET,
            cast::<_, [u8; 4]>([rect_y.to_be(), rect_end_y.to_be()]),
        );
        self.send_cmd(CO5300_W_RAMWR, []);
        self.cs_pin.set_low();
        // first write has command and address, subsequent writes do not
        let mut is_first_write = true;
        while total_pixels_to_send > 0 {
            // the amount of pixels currently being sent, has max of buffer size
            let current_tx_pixels_to_send = total_pixels_to_send.min(Self::MAX_PIXELS_SENT_AT_ONCE);
            self.pixel_buf.clear();

            for i in 0..current_tx_pixels_to_send {
                let (px, py) = (
                    rect_x as u32 + (i + sent_pixels) % rect_w as u32,
                    rect_y as u32 + (i + sent_pixels) / rect_w as u32,
                );
                self.pixel_buf
                    .push(*pixel_fn(px as u16, py as u16))
                    .unwrap();
            }
            let (qspi_command, qspi_address) = if is_first_write {
                is_first_write = false;
                (
                    Command::_8Bit(0x32, DataMode::Single),
                    Address::_24Bit(0x003C00, DataMode::Single),
                )
            } else {
                (Command::None, Address::None)
            };
            self.spi
                .half_duplex_write(
                    DataMode::Quad,
                    qspi_command,
                    qspi_address,
                    0,
                    cast_slice(&self.pixel_buf),
                )
                .unwrap();
            sent_pixels += current_tx_pixels_to_send;
            total_pixels_to_send -= current_tx_pixels_to_send;
        }
        self.cs_pin.set_high();
    }

    /// rect x, y, width, and height must all be even (co5300 restrictions)
    pub fn draw_buf(
        &mut self,
        rect_x: u16,
        rect_y: u16,
        rect_w: u16,
        rect_h: u16,
        buf: &[RGB565],
    ) {
        debug_assert!(rect_x % 2 == 0);
        debug_assert!(rect_y % 2 == 0);
        debug_assert!(rect_w % 2 == 0);
        debug_assert!(rect_h % 2 == 0);
        debug_assert!((rect_w * rect_h) as usize == buf.len());

        let rect_end_x = rect_x + rect_w - 1;
        let rect_end_y = rect_y + rect_h - 1;
        let mut total_pixels_to_send = rect_w as u32 * rect_h as u32;
        self.send_cmd(
            CO5300_W_CASET,
            cast::<_, [u8; 4]>([
                (rect_x + Self::COL_OFFSET).to_be(),
                (rect_end_x + Self::COL_OFFSET).to_be(),
            ]),
        );
        self.send_cmd(
            CO5300_W_PASET,
            cast::<_, [u8; 4]>([rect_y.to_be(), rect_end_y.to_be()]),
        );
        self.send_cmd(CO5300_W_RAMWR, []);
        self.cs_pin.set_low();
        // first write has command and address, subsequent writes do not
        let mut is_first_write = true;
        while total_pixels_to_send > 0 {
            // the amount of pixels currently being sent, has max of buffer size
            let current_tx_pixels_to_send = total_pixels_to_send.min(Self::MAX_PIXELS_SENT_AT_ONCE);
            self.pixel_buf.clear();
            
            let (qspi_command, qspi_address) = if is_first_write {
                is_first_write = false;
                (
                    Command::_8Bit(0x32, DataMode::Single),
                    Address::_24Bit(0x003C00, DataMode::Single),
                )
            } else {
                (Command::None, Address::None)
            };
            self.spi
                .half_duplex_write(
                    DataMode::Quad,
                    qspi_command,
                    qspi_address,
                    0,
                    cast_slice(&buf),
                )
                .unwrap();
            total_pixels_to_send -= current_tx_pixels_to_send;
        }
        self.cs_pin.set_high();
    }
}
