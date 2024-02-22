//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to specified Wifi network and creates a TCP endpoint on port 1234.

#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use picoserve::extract::State;
use picoserve::response::IntoResponse;
use picoserve::routing::get;
use rand::Rng;
use static_cell::make_static;
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

use stemma_soil_sensor_embassy::SoilSensor;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = env!("SSID");
const WIFI_PASSWORD: &str = env!("PASSWORD");

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

type AppRouter = impl picoserve::routing::PathRouter<AppState>;

const WEB_TASK_POOL_SIZE: usize = 8;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: &'static embassy_net::Stack<cyw43::NetDriver<'static>>,
    app: &'static picoserve::Router<AppRouter, AppState>,
    config: &'static picoserve::Config<Duration>,
    state: AppState,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve_with_state(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
        &state,
    )
    .await
}

#[derive(Clone, Copy)]
struct SharedSoilSensor(&'static Mutex<CriticalSectionRawMutex, SoilSensor<'static>>);

struct AppState {
    shared_soil_sensor: SharedSoilSensor,
}

impl picoserve::extract::FromRef<AppState> for SharedSoilSensor {
    fn from_ref(state: &AppState) -> Self {
        state.shared_soil_sensor
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Init network stack
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<WEB_TASK_POOL_SIZE>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<WEB_TASK_POOL_SIZE>::new()),
        embassy_rp::clocks::RoscRng.gen(),
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    // And now we can use it!

    let soil_sensor = SoilSensor::new(p.PIN_4, p.PIN_5, p.I2C0).await.unwrap();
    let shared_soil_sensor = SharedSoilSensor(make_static!(Mutex::new(soil_sensor)));

    fn make_app() -> picoserve::Router<AppRouter, AppState> {
        picoserve::Router::new()
            .route("/", get(|| async move { "Hello World" }))
            .route(
                "/moisture",
                get(|State(SharedSoilSensor(sensor))| async move {
                    let mut sensor = sensor.lock().await;
                    sensor.read_moisture().await.map(|m| Moisture {
                        val: m,
                        json: false,
                    })
                }),
            )
            .route(
                "/temp",
                get(|State(SharedSoilSensor(sensor))| async move {
                    let mut sensor = sensor.lock().await;
                    sensor.read_temperature().await.map(|t| Temperature {
                        val: t,
                        json: false,
                    })
                }),
            )
            .route(
                "/api/moisture",
                get(|State(SharedSoilSensor(sensor))| async move {
                    let mut sensor = sensor.lock().await;
                    sensor
                        .read_moisture()
                        .await
                        .map(|m| Moisture { val: m, json: true })
                }),
            )
            .route(
                "/api/temp",
                get(|State(SharedSoilSensor(sensor))| async move {
                    let mut sensor = sensor.lock().await;
                    sensor
                        .read_temperature()
                        .await
                        .map(|t| Temperature { val: t, json: true })
                }),
            )
    }

    let app = make_static!(make_app());

    info!("Starting web server");

    let config = make_static!(picoserve::Config::new(picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    })
    .keep_connection_alive());

    // for some reason, idk why, I can only spawn one less than the pool size
    // otherwise it panics
    for id in 1..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(
            id,
            stack,
            app,
            config,
            AppState { shared_soil_sensor },
        ));
    }

    info!("Web server started");
}

struct Temperature<T: core::fmt::Display> {
    pub val: T,
    pub json: bool,
}

impl<T: core::fmt::Display> IntoResponse for Temperature<T> {
    async fn write_to<
        R: picoserve::io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        self,
        connection: picoserve::response::Connection<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if self.json {
            format_args!("{{\"temperature\":{}}}", self.val)
                .write_to(connection, response_writer)
                .await
        } else {
            format_args!("Temperature: {}°C", self.val)
                .write_to(connection, response_writer)
                .await
        }
    }
}

struct Moisture {
    pub val: u16,
    pub json: bool,
}

impl IntoResponse for Moisture {
    async fn write_to<
        R: picoserve::io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        self,
        connection: picoserve::response::Connection<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if self.json {
            format_args!("{{\"moisture\":{}}}", self.val)
                .write_to(connection, response_writer)
                .await
        } else {
            format_args!("Moisture: {}", self.val)
                .write_to(connection, response_writer)
                .await
        }
    }
}
