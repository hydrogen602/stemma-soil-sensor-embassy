use embassy_rp::i2c;
use thiserror_no_std as thiserror;

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SetupError {
    #[error("I2C error: {0}")]
    I2C(#[from] I2CError),
    #[error("Invalid device: {hw_id}")]
    InvalidDevice { hw_id: u8 },
}

#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[error("I2C error: {0:?}")]
pub struct I2CError(pub i2c::AbortReason);

pub type I2CResult<T> = core::result::Result<T, I2CError>;

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
