//! Error types for the soil sensor library

use embassy_rp::i2c;
use thiserror_no_std as thiserror;

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
/// Errors that can occur when setting up the soil sensor, or when trying to read from it
pub enum SetupError {
    /// Something went wrong with I2C
    #[error("I2C error: {0}")]
    I2C(#[from] I2CError),
    /// The sensor returned an invalid hardware ID.
    /// This suggests that the sensor is not connected, is not functioning correctly, or is not the Stemma Soil Sensor.
    #[error("Invalid device: {hw_id}")]
    InvalidDevice { hw_id: u8 },
    /// The I2C address is invalid. The Stemma Soil Sensor can only be configured to use
    /// one of four addresses: 0x36, 0x37, 0x38, or 0x39.
    /// See the [Adafruit documentation](https://learn.adafruit.com/adafruit-stemma-soil-sensor-i2c-capacitive-moisture-sensor/pinouts)
    #[error("Invalid I2C address: {address}. The sensor address must be between 0x36 and 0x39, inclusive.")]
    InvalidI2CAddress { address: u8 },
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[error("I2C error: {0:?}")]
/// Errors that can occur when trying to read from or write to the I2C bus
pub struct I2CError(pub i2c::AbortReason);

/// A type alias for the result of an I2C operation
pub type I2CResult<T> = core::result::Result<T, I2CError>;

/// A type alias for the result of setting up the soil sensor
pub type SetupResult<T> = core::result::Result<T, SetupError>;

impl From<i2c::Error> for I2CError {
    /// Sort out the different I2C errors into those that are user-caused
    /// and those that are bugs in the library
    fn from(e: i2c::Error) -> Self {
        match e {
            i2c::Error::Abort(reason) => I2CError(reason),
            i2c::Error::InvalidReadBufferLength => {
                core::panic!("InvalidReadBufferLength. This should never occur and is a bug.")
            }
            i2c::Error::InvalidWriteBufferLength => {
                core::panic!("InvalidWriteBufferLength. This should never occur and is a bug.")
            }
            i2c::Error::AddressOutOfRange(addr) => {
                core::panic!(
                    "InvalidAddress({}). This should never occur and is a bug.",
                    addr
                )
            }
            i2c::Error::AddressReserved(addr) => {
                core::panic!(
                    "AddressReserved({}). This should never occur and is a bug.",
                    addr
                )
            }
        }
    }
}

#[cfg(feature = "picoserve")]
/// This allows returning an I2CError as a response to an HTTP request
impl picoserve::response::IntoResponse for I2CError {
    async fn write_to<
        R: picoserve::io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        self,
        connection: picoserve::response::Connection<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        response_writer
            .write_response(
                connection,
                picoserve::response::Response::new(
                    picoserve::response::StatusCode::new(500),
                    format_args!("{}", self),
                ),
            )
            .await
    }
}

#[cfg(feature = "picoserve")]
/// This allows returning a SetupError as a response to an HTTP request
impl picoserve::response::IntoResponse for SetupError {
    async fn write_to<
        R: picoserve::io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        self,
        connection: picoserve::response::Connection<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        response_writer
            .write_response(
                connection,
                picoserve::response::Response::new(
                    picoserve::response::StatusCode::new(500),
                    format_args!("{}", self),
                ),
            )
            .await
    }
}
