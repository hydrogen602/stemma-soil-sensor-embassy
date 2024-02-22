#![no_std]
#![allow(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]

pub mod error;
mod soil_sensor;

pub use soil_sensor::SoilSensor;
