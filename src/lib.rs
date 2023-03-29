// Copyright 2022, Ivan Palijan <ivan95.603@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/license/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT> or BSD, at your
// option.  This file may not be copied, modified, or distributed
// except according to those terms.

// This is the no-std, embedded-hal library for the Bosh BMP180 i2c temperature and pressure sensor.

//! A platform agnostic driver to interface with the BMP180 (Pressure sensor)
//!
//! This driver was built using [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal

//! # Example
//!  ```
//! #![no_std]
//! #![no_main]
//! // For allocator
//! #![feature(lang_items)]
//! #![feature(alloc_error_handler)]

//! use alloc::{string::ToString, boxed::Box};
//! use cortex_m::{asm, delay::Delay};
//! use cortex_m_rt::{exception, entry, ExceptionFrame};

//! use core::alloc::Layout;

//! use cortex_m;

//! use stm32f4::stm32f411::I2C1;

//! use stm32f4xx_hal as hal;

//! use bmp180_nostd as bmp180;

//! use crate::hal::{
//!     pac,
//!     pac::{Peripherals},
//!     i2c::I2c,
//!     block,
//!     prelude::*,
//!     serial::config::Config,
//!     serial::Serial, serial::*,
//!     gpio,
//!     gpio::{
//!         Alternate,
//!         gpiob,
//!     },
//! };

//! use core::fmt::{Error, Write}; // for pretty formatting of the serial output

//! #[macro_use]
//! extern crate alloc;

//! type I2C1Bus = I2c<I2C1, (stm32f4xx_hal::gpio::Pin<Alternate<OpenDrain, 4_u8>, 'B', 8_u8>, stm32f4xx_hal::gpio::Pin<Alternate<OpenDrain, 4_u8>, 'B', 9_u8>)>;

//! #[entry]
//! fn main() -> ! {
//!     let dp = Peripherals::take().unwrap();
//!     let cp = cortex_m::Peripherals::take().unwrap();

//!     let rcc = dp.RCC.constrain();
//!     let clocks = rcc
//!                         .cfgr
//!                         .use_hse(25.MHz())
//!                         .sysclk(100.MHz())
//!                         .pclk1(50.MHz())
//!                         .pclk2(100.MHz())
//!                         .freeze();  

//!     // // Create a delay abstraction based on SysTick
//!     // let mut delayObj = hal::delay::Delay::new(cp.SYST, &clocks);

//!     // // Create a delay abstraction based on general-pupose 32-bit timer TIM5
//!     let mut delayObj  = dp.TIM5.delay_us(&clocks);

//!     let mut gpiob = dp.GPIOB.split();
//!     let scl = gpiob
//!         .pb8
//!         .into_alternate()
//!         .internal_pull_up(true)
//!         .set_open_drain();
//!     let sda = gpiob
//!         .pb9
//!         .into_alternate()
//!         .internal_pull_up(true)
//!         .set_open_drain();

//!     delayObj.delay_ms(100 as u32);

//!     let i2c = I2c::new(dp.I2C1, (scl, sda), 100.kHz(), &clocks); //100 kHz I2C Bus speed

//!     let mut boxy: Box<dyn embedded_hal::blocking::delay::DelayMs<u32>+ core::marker::Send> = Box::new(delayObj);

//!     // The bus is a 'static reference -> it lives forever and references can be
//!     // shared with other threads.
//!     // let bus: &'static _ = shared_bus::new_cortexm!(I2C1Bus = i2c).unwrap();
//!     // let mut barometer = bmp180::BMP180BarometerThermometer::new(bus.acquire_i2c(), boxy , bmp180::BMP180PressureMode::BMP180Standard);

//!     let mut barometer = bmp180::BMP180BarometerThermometer::new(i2c, boxy , bmp180::BMP180PressureMode::BMP180Standard);

//!     let pressure_in_hpa: f32 = barometer.pressure_hpa();

//!     let pressure_temp_celsius: f32 = barometer.temperature_celsius();
//! }

//! ```

#![allow(dead_code)]
#![deny(missing_docs)]
#![deny(warnings)]

extern crate alloc;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ByteOrder};
use embedded_hal as hal;
use std::time::Duration;

/// BMP180 Default device address
pub const BMP180_I2C_ADDR: u8 = 0x77;

/// Measurement control register address
pub const BMP180_REGISTER_CTL: u8 = 0xF4;
/// Temperature command
pub const BMP180_CMD_TEMP: u8 = 0x2E;
/// Temperature register MSB address
pub const BMP180_REGISTER_TEMP_MSB: u8 = 0xF6;
/// Pressure CMD 4.5ms conversion time
pub const BMP180_CMD_PRESSURE: u8 = 0x34;
/// Pressure register MSB address
pub const BMP180_REGISTER_PRESSURE_MSB: u8 = 0xF6;
/// Calibration register address AC1 MSB
pub const BMP180_REGISTER_AC1MSB: u8 = 0xaa;

/// All possible errors in this crate
#[derive(Debug)]
pub enum Error<E> {
    /// I²C bus error
    I2C(E),
    /// CRC checksum mismatch (PEC)
    ChecksumMismatch,
    /// Invalid input data
    InvalidInputData,
}

/// BMP180 Hardware pressure sampling accuracy modes.
#[derive(Copy, Clone)]
pub enum BMP180PressureMode {
    /// Ultra Low Power mode. Internal number of samples: 1, Max conversion time: 4.5ms, Current: 3uA, RMS noise: 0.06 hPa, 0.5m.
    BMP180UltraLowPower,
    /// Standard mode. Internal number of samples: 2, Max conversion time: 7.5ms, Current: 5uA, RMS noise: 0.05 hPa, 0.4m.
    BMP180Standard,
    /// High Resolution mode. Internal number of samples: 4, Max conversion time: 13.5ms, Current: 7uA, RMS noise: 0.04 hPa, 0.3m.
    BMP180HighResolution,
    /// Ultra High Resolution mode. Internal number of samples: 8, Max conversion time: 25.5ms, Current: 12uA, RMS noise: 0.03 hPa, 0.25m.
    BMP180UltraHighResolution,
}

impl BMP180PressureMode {
    /// Returns selected hardware pressure sampling accuracy mode.
    pub fn get_mode_value(self) -> u8 {
        match self {
            BMP180PressureMode::BMP180UltraLowPower => 0,
            BMP180PressureMode::BMP180Standard => 1,
            BMP180PressureMode::BMP180HighResolution => 2,
            BMP180PressureMode::BMP180UltraHighResolution => 3,
        }
    }
    /// Returns selected hardware pressure sampling accuracy delay in ms.
    pub fn mode_delay(self) -> u8 {
        match self {
            BMP180PressureMode::BMP180UltraLowPower => 5,
            BMP180PressureMode::BMP180Standard => 8,
            BMP180PressureMode::BMP180HighResolution => 14,
            BMP180PressureMode::BMP180UltraHighResolution => 26,
        }
    }
}
///Calibration coefficients
#[derive(Copy, Clone)]
pub struct BMP180CalibrationCoefficients {
    ac1: i16,
    ac2: i16,
    ac3: i16,
    ac4: u16,
    ac5: u16,
    ac6: u16,
    b1: i16,
    b2: i16,
    mb: i16,
    mc: i16,
    md: i16,
}

///Structure that holds raw BMP180 pressure and temerature reading.
#[derive(Debug)]
pub struct BMP180RawReading {
    padc: i32, // 10-bit pressure ADC output value
    tadc: i16, // 10-bit pressure ADC output value
}
//  #[derive(Clone)]
#[allow(non_snake_case)]
/// BMP180 sensor struct
pub struct BMP180BarometerThermometer<I2C>
where
    I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
{
    /// Holds I2C bus
    pub i2c: I2C,
    /// Holds factory calibration coefficients.
    pub coeff: BMP180CalibrationCoefficients,
    /// Holds chosen pressure mode.
    pub pressure_precision: BMP180PressureMode,
    last_reading: Option<BMP180RawReading>,
}

#[allow(non_snake_case)]
impl<I2C> BMP180BarometerThermometer<I2C>
where
    I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
{
    /// Create sensor accessor for BMP180 on the provided i2c bus path
    pub fn new(
        mut i2c: I2C,
        pressure_precision: BMP180PressureMode,
    ) -> BMP180BarometerThermometer<I2C> {
        let coeff = BMP180CalibrationCoefficients::new(&mut i2c);
        BMP180BarometerThermometer {
            i2c: i2c,
            coeff: coeff,
            pressure_precision: pressure_precision,
            last_reading: None,
        }
    }

    /// Read raw pressure and temperature from sensor
    pub async fn update(&mut self) -> Result<()> {
        let reading = self.raw_reading(self.pressure_precision).await?;
        self.last_reading = Some(reading);
        Ok(())
    }

    /// Return measured pressure in pascals
    ///
    /// You MUST call `update` before calling this function
    pub fn pressure_pa(&self) -> Option<f32> {
        if let Some(reading) = &self.last_reading {
            let b5 = self.coeff.calculate_b5(reading.tadc);
            let real_pressure =
                calculate_real_pressure(reading.padc, b5, self.coeff, self.pressure_precision);
            Some(real_pressure as f32)
        } else {
            None
        }
    }

    /// Return measured pressure in hPa
    ///
    /// You MUST call `update` before calling this function
    pub fn pressure_hpa(&mut self) -> Option<f32> {
        let p = self.pressure_pa()?;
        Some(p / 100_f32)
    }

    /// Return measured pressure in kilopascal
    ///
    /// You MUST call `update` before calling this function
    pub fn pressure_kpa(&mut self) -> Option<f32> {
        let p = self.pressure_pa()?;
        Some(p / 1000_f32)
    }

    /// Return measured RAW reading
    async fn raw_reading(&mut self, mode: BMP180PressureMode) -> Result<BMP180RawReading> {
        // fist we need read temp needed for further pressure calculations
        self.i2c
            .write(BMP180_I2C_ADDR, &[BMP180_REGISTER_CTL, BMP180_CMD_TEMP])
            .map_err(|_| anyhow!("write error"))?;

        // maximum conversion time is 5ms
        tokio::time::sleep(Duration::from_millis(5)).await;
        // Read uncompensated temperature (two registers)
        // i2c gets LittleEndian we need BigEndian
        let mut buf = [0_u8; 2];
        self.i2c
            .write_read(BMP180_I2C_ADDR, &[BMP180_REGISTER_TEMP_MSB], &mut buf)
            .map_err(|_| anyhow!("write read error"))?;
        // let _ = self.i2c.read(BMP180_I2C_ADDR, &mut buf);
        // we have raw temp data in tadc.
        let tadc: i16 = BigEndian::read_i16(&buf[..]);
        // println!("Raw Temp: {}", tadc);
        // now lets get pressure
        let offset = mode.get_mode_value();
        let delay = mode.mode_delay();
        self.i2c
            .write(
                BMP180_I2C_ADDR,
                &[BMP180_REGISTER_CTL, BMP180_CMD_PRESSURE + (offset << 6)],
            )
            .map_err(|_| anyhow!("write error"))?;
        tokio::time::sleep(Duration::from_millis(delay as u64)).await;
        let mut p_buf = [0_u8; 3];
        self.i2c
            .write_read(BMP180_I2C_ADDR, &[BMP180_REGISTER_PRESSURE_MSB], &mut p_buf)
            .map_err(|_| anyhow!("write read error"))?;
        // let _ = self.i2c.read(BMP180_I2C_ADDR, &mut p_buf);
        let padc: i32 = (((p_buf[0] as i32) << 16) + ((p_buf[1] as i32) << 8) + (p_buf[2] as i32))
            >> (8 - (offset as u8));
        Ok(BMP180RawReading {
            padc: padc,
            tadc: tadc,
        })
    }
    /// Return temperature measurement from BMP180
    /// 
    /// You MUST call `update` before calling this function
    pub fn temperature_celsius(&mut self) -> Option<f32> {
        if let Some(reading) = &self.last_reading {
            let b5 = self.coeff.calculate_b5(reading.tadc);
            let t = (b5 + 8) >> 4;
            Some((t as f32) / 10_f32)
        } else {
            None
        }
    }
}

#[allow(non_snake_case)]
impl BMP180CalibrationCoefficients {
    /// Creates struct object that holds factory calibration corefficients
    pub fn new<I2C>(i2c: &mut I2C) -> BMP180CalibrationCoefficients
    where
        I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
    {
        let mut buf: [u8; 22] = [0; 22];
        let _ = i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_AC1MSB]);
        let _ = i2c.read(BMP180_I2C_ADDR, &mut buf);
        // unimplemented!();
        BMP180CalibrationCoefficients {
            ac1: BigEndian::read_i16(&buf[0..2]),
            ac2: BigEndian::read_i16(&buf[2..4]),
            ac3: BigEndian::read_i16(&buf[4..6]),
            ac4: BigEndian::read_u16(&buf[6..8]),
            ac5: BigEndian::read_u16(&buf[8..10]),
            ac6: BigEndian::read_u16(&buf[10..12]),
            b1: BigEndian::read_i16(&buf[12..14]),
            b2: BigEndian::read_i16(&buf[14..16]),
            mb: BigEndian::read_i16(&buf[16..18]),
            mc: BigEndian::read_i16(&buf[18..20]),
            md: BigEndian::read_i16(&buf[20..22]),
        }
    }
    fn calculate_b5(self, raw_temp: i16) -> i32 {
        let x1 = (((raw_temp as i32) - (self.ac6 as i32)) * (self.ac5 as i32)) >> 15;
        let x2 = ((self.mc as i32) << 11) / (x1 + (self.md as i32));
        x1 + x2
    }
}

fn calculate_real_pressure(
    padc: i32,
    b5: i32,
    coeff: BMP180CalibrationCoefficients,
    oss: BMP180PressureMode,
) -> f32 {
    let b6: i32 = b5 - 4000i32;

    let _t = (b6 as i32).pow(2) >> 12;
    let mut x1: i32 = (coeff.b2 as i32 * _t) >> 11;
    let mut x2: i32 = (coeff.ac2 as i32 * b6) >> 11;
    let x3: u32 = (x1 + x2) as u32;
    let b3: i32 = (((coeff.ac1 as i32 * 4 + (x3 as i32)) << oss.get_mode_value()) + 2) / 4;
    x1 = (coeff.ac3 as i32 * b6) >> 13;
    x2 = (coeff.b1 as i32 * _t) >> 16;
    let x3: i32 = (x1 + x2 + 2) >> 2;

    let _x3: u32 = (x3 + 32768i32) as u32;
    let b4: u32 = (coeff.ac4 as u32 * _x3) >> 15;
    let b7: u32 = (padc - b3) as u32 * (50000 >> oss.get_mode_value());
    let p = if b7 < 0x80000000 {
        (b7 << 1) / b4
    } else {
        (b7 / b4) << 1
    } as i32;

    x1 = (p >> 8).pow(2);
    x1 = (x1 * 3038) >> 16;
    x2 = (-7357 * (p)) >> 16;

    ((p) + ((x1 + x2 + 3791) >> 4)) as f32 // return as Pa
}
