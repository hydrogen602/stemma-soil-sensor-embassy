//! A library for interacting with Adafruit's STEMMA soil sensor.
//! The library works by creating a `SoilSensor` object,
//! which can then be used to read the temperature and moisture from the sensor.

#![no_std]
#![allow(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]

pub mod error;
mod soil_sensor;

/// The soil sensor object. It is the main entry point for the library
/// and is used to interact with the soil sensor.
pub use soil_sensor::SoilSensor;
