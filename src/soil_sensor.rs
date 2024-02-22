// Derived from https://github.com/adafruit/Adafruit_CircuitPython_seesaw/

use embassy_rp::i2c;
use embassy_rp::peripherals::{I2C0, PIN_4, PIN_5};
use embassy_time::Timer;
use embedded_hal_1::i2c::I2c;

use crate::error::{I2CResult, SetupError, SetupResult};

const MOISTURE_DELAY: u64 = 5000;
const TEMP_DELAY: u64 = 125;
const HW_ID_DELAY: u64 = 125;

const TEMP_C_CONSTANT: f32 = 0.00001525878;

pub struct SoilSensor<'i> {
    i2c: i2c::I2c<'i, I2C0, i2c::Blocking>,
    address: u8,
}

impl SoilSensor<'_> {
    pub async fn new(sda: PIN_4, scl: PIN_5, i2c: I2C0) -> SetupResult<Self> {
        let i2c: i2c::I2c<'_, I2C0, i2c::Blocking> =
            i2c::I2c::new_blocking(i2c, scl, sda, i2c::Config::default());

        let mut sensor = SoilSensor { i2c, address: 0x36 };
        let hw_id = sensor.read_hw_id().await?;

        if hw_id == seesaw::SENSOR_HW_ID_CODE {
            Ok(sensor)
        } else {
            Err(SetupError::InvalidDevice { hw_id })
        }
    }

    pub async fn read_temperature(&mut self) -> I2CResult<i32> {
        let mut buf = [0; 4];
        self.i2c_read(
            &[seesaw::STATUS_BASE, seesaw::STATUS_TEMP],
            &mut buf,
            TEMP_DELAY,
        )
        .await?;
        let raw = i32::from_be_bytes(buf);

        // rounding to the nearest integer (+1/2, then cast)
        // the sensor is maybe good to + or - 2 degrees Celsius, so the decimal places are meaningless
        // https://www.adafruit.com/product/4026
        Ok((raw as f32 * TEMP_C_CONSTANT + 0.5) as i32)
    }

    pub async fn read_moisture(&mut self) -> I2CResult<u16> {
        let mut buf = [0; 2];

        self.i2c_read(
            &[seesaw::TOUCH_BASE, seesaw::TOUCH_CHANNEL_OFFSET],
            &mut buf,
            MOISTURE_DELAY,
        )
        .await?;

        Ok(u16::from_be_bytes(buf))
    }

    pub async fn read_hw_id(&mut self) -> I2CResult<u8> {
        let mut buf = [0; 1];
        self.i2c_read(
            &[seesaw::STATUS_BASE, seesaw::STATUS_HW_ID],
            &mut buf,
            HW_ID_DELAY,
        )
        .await?;

        Ok(buf[0])
    }

    async fn i2c_read(&mut self, bytes: &[u8], buffer: &mut [u8], delay_us: u64) -> I2CResult<()> {
        self.i2c.write(self.address, bytes)?;
        Timer::after_micros(delay_us).await;
        self.i2c.read(self.address, buffer)?;
        Ok(())
    }
}

mod seesaw {
    pub const SENSOR_HW_ID_CODE: u8 = 0x55;

    pub const STATUS_BASE: u8 = 0x00;
    pub const TOUCH_BASE: u8 = 0x0F;

    pub const STATUS_HW_ID: u8 = 0x01;
    pub const STATUS_TEMP: u8 = 0x04;

    pub const TOUCH_CHANNEL_OFFSET: u8 = 0x10;
}
