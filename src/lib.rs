// Copyright 2022, Ivan Palijan <ivan95.603@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/license/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option.  This file may not be copied, modified, or distributed
// except according to those terms.

// This module is implementing userland library to interact with
// Bosh BMP180 i2c temperature and pressure sensor.

//! A platform agnostic driver to interface with the BMP180 (Pressure sensor)
//!
//! This driver was built using [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal

// #![allow(dead_code)]
// #![deny(missing_docs)]
// #![deny(warnings)]
#![no_std]

use embedded_hal as hal;
use hal::blocking::i2c;
use byteorder;

extern crate alloc;
use alloc::boxed::Box;

// use i2csensors::{Barometer,Thermometer};
// use std::thread;
// use std::time::Duration;
// use std::error::Error;
// use i2cdev::core::I2CDevice;
use byteorder::{ByteOrder, BigEndian};

pub const BMP180_I2C_ADDR: u8 = 0x77;

pub const BMP180_REGISTER_CTL: u8 = 0xF4;
pub const BMP180_CMD_TEMP: u8 = 0x2E;
pub const BMP180_REGISTER_TEMP_MSB: u8 = 0xF6;
pub const BMP180_CMD_PRESSURE: u8 = 0x34;
pub const BMP180_REGISTER_PRESSURE_MSB: u8 = 0xF6;
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

#[derive(Copy,Clone)]
pub enum BMP180PressureMode {
    BMP180UltraLowPower,
    BMP180Standard,
    BMP180HighResolution,
    BMP180UltraHighResolution,
}

impl BMP180PressureMode {
    pub fn get_mode_value(self) -> u8 {
        match self {
            BMP180PressureMode::BMP180UltraLowPower => 0,
            BMP180PressureMode::BMP180Standard => 1,
            BMP180PressureMode::BMP180HighResolution => 2,
            BMP180PressureMode::BMP180UltraHighResolution => 3,
        }
    }
    pub fn mode_delay(self) -> u8 {
        match self {
            BMP180PressureMode::BMP180UltraLowPower => 5,
            BMP180PressureMode::BMP180Standard => 8,
            BMP180PressureMode::BMP180HighResolution => 14,
            BMP180PressureMode::BMP180UltraHighResolution => 26,

            // BMP180PressureMode::BMP180UltraLowPower => Duration::from_millis(5),
            // BMP180PressureMode::BMP180Standard => Duration::from_millis(8),
            // BMP180PressureMode::BMP180HighResolution => Duration::from_millis(14),
            // BMP180PressureMode::BMP180UltraHighResolution => Duration::from_millis(26),
        }
    }
}
#[derive(Copy,Clone)]
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

#[derive(Debug)]
pub struct BMP180RawReading {
    padc: i32, // 10-bit pressure ADC output value
    tadc: i16, // 10-bit pressure ADC output value
}
//  #[derive(Clone)]
pub struct BMP180BarometerThermometer<'a, I2C>
where 
    I2C:hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
{
    pub i2c: I2C,
    delayObj: Box<&'a mut (dyn embedded_hal::blocking::delay::DelayMs<u32> + core::marker::Send)>,
    pub coeff: BMP180CalibrationCoefficients,
    pub pressure_precision: BMP180PressureMode,
}


impl<'a, I2C> BMP180BarometerThermometer<'a, I2C>
where
    I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
{
    /// Create sensor accessor for MPL115A2 on the provided i2c bus path
    pub fn new(mut i2c: I2C, delayObj: Box<&'a mut (dyn embedded_hal::blocking::delay::DelayMs<u32>+ core::marker::Send)>, pressure_precision: BMP180PressureMode) -> BMP180BarometerThermometer<I2C> {
        let coeff = BMP180CalibrationCoefficients::new(&mut i2c);
        BMP180BarometerThermometer {
            i2c: i2c,
            delayObj: delayObj,
            coeff: coeff,
            pressure_precision: pressure_precision,
        }
    }
    pub fn pressure_pa(&mut self) -> f32 {
        let reading = &self.BMP180RawReading(self.pressure_precision);
        let b5 = self.coeff.calculate_b5(reading.tadc);
        let real_pressure = calculate_real_pressure(reading.padc, b5, self.coeff, self.pressure_precision);
        return real_pressure as f32
    }
    pub fn pressure_hpa(&mut self) -> f32 {
        self.pressure_pa()  / 100_f32
    }




    pub fn BMP180RawReading(&mut self, mode: BMP180PressureMode) -> BMP180RawReading {
        // fist we need read temp needed for further pressure calculations
        self.i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_CTL, BMP180_CMD_TEMP]);

        // maximum conversion time is 5ms
        self.delayObj.delay_ms(5 as u32);
        // thread::sleep(Duration::from_millis(5));
        // Read uncompensated temperature (two registers)
        // i2c gets LittleEndian we need BigEndian
        let mut buf = [0_u8; 2];
        self.i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_TEMP_MSB]);
        self.i2c.read(BMP180_I2C_ADDR, &mut buf);
        // we have raw temp data in tadc.








        let tadc: i16 = BigEndian::read_i16(&buf[..]);








        // println!("Raw Temp: {}", tadc);
        // now lets get pressure
        let offset = mode.get_mode_value();
        let delay = mode.mode_delay();
        self.i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_CTL, BMP180_CMD_PRESSURE + (offset << 6)]);
        self.delayObj.delay_ms(delay as u32);
        let mut p_buf = [0_u8; 3];
        self.i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_PRESSURE_MSB]);
        self.i2c.read(BMP180_I2C_ADDR, &mut p_buf);
        let padc: i32 = (((p_buf[0] as i32) << 16) + ((p_buf[1] as i32) << 8) + (p_buf[2] as i32)) >> (8 - (offset as u8));
        BMP180RawReading {
            padc: padc,
            tadc: tadc,
        }
    }


}



impl BMP180CalibrationCoefficients 
{
    pub fn new<I2C>(i2c: &mut I2C) -> BMP180CalibrationCoefficients 
    where
        I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
    {
        let mut buf: [u8; 22] = [0; 22];
        i2c.write(BMP180_I2C_ADDR, &[BMP180_REGISTER_AC1MSB]);
        i2c.read(BMP180_I2C_ADDR, &mut buf);
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

// impl<I2C> Thermometer for BMP180BarometerThermometer<I2C>
//     where 
//     I2C: hal::blocking::i2c::WriteRead + hal::blocking::i2c::Write + hal::blocking::i2c::Read,
// {
//     // type Error = T::Error;

//     fn temperature_celsius(&mut self) -> f32{
//         let reading = &self.BMP180RawReading(self.pressure_precision);
//         let b5 = self.coeff.calculate_b5(reading.tadc);
//         let t = (b5 + 8) >> 4;
//         (t as f32) / 10_f32
//     }
// }

// impl<I2C> Barometer for BMP180BarometerThermometer<I2C>
//     where 
//     I2C: hal::blocking::i2c::WriteRead<hal::blocking::i2c::SevenBitAddress> + hal::blocking::i2c::Write<hal::blocking::i2c::SevenBitAddress>  + Sized,
// {
//     // type Error = T::Error;

//     fn pressure_kpa(&mut self) -> f32 {
//         return self.pressure_pa() / 1000_f32;
//     }
// }

fn calculate_real_pressure(padc: i32, b5: i32, coeff: BMP180CalibrationCoefficients, oss: BMP180PressureMode) -> f32 {
    // welcome to the hardware world :)
    // this code is exact formula from BMP180 datasheet page 15 (Figure 4)
    // written in Rust
    let b6: i32 = b5 - 4000;
    let mut x1: i32 = ((coeff.b2 as i32) * ((b6 * b6) / (2 << 11))) / (2 << 10);
    let mut x2: i32 = ((coeff.ac2 as i32) * b6) >> 11;
    let mut x3: i32 = x1 + x2;
    let b3 = ((((coeff.ac1 as u32) * 4 + x3 as u32) << oss.get_mode_value()) + 2) >> 2;
    x1 = ((coeff.ac3 as i32) * b6) >> 13;
    x2 = ((coeff.b1 as i32) * ((b6 * b6) >> 12)) >> 16;
    x3 = ((x1 + x2) + 2) >> 2;
    let b4 = ((coeff.ac4 as u32) * (x3 as u32 + 32768)) >> 15;
    let b7 = ((padc as u32 - b3 as u32) as u32) * (50000 >> oss.get_mode_value());
    let mut p: i32;
    if b7 < 0x80000000 {
        p = ((b7 * 2) / b4) as i32;
    } else {
        p = ((b7 / b4) * 2) as i32;
    }
    x1 = ((p >> 8) * (p >> 8)) as i32;
    x1 = (x1 * 3038) >> 16;
    x2 = (-7357_i32 * p) >> 16;
    (p + ((x1 + x2 + 3791) >> 4) as i32) as f32
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     // use sensors::*;
//     use mock::MockI2CDevice;
//     pub const BMP180_REGISTER_PRESSURE_MSB_TEST: usize = 0x90;
//     macro_rules! assert_almost_eq {
//         ($left:expr, $right:expr) => ({
//             match (&($left), &($right)) {
//                 (left_val, right_val) => {
//                     if (*left_val - *right_val).abs() > 0.0001 {
//                         panic!("assertion failed: ({:?} != {:?})", *left_val, *right_val);
//                     }
//                 }
//             }
//         })
//     }

//     // BMP180 device holds pressure and temp value in the same register
//     // what is stored there is depending on what will be written to BMP180_REGISTER_CTL
//     // before reading common 0xf6 register
//     // testing with I2C Mockup requires some trickery :)
//     // test values are taken from BMP180 datasheet page 15 (Figure 4)
//     fn make_dev(mut i2cdev: MockI2CDevice) -> BMP180BarometerThermometer<MockI2CDevice> {
//         (&mut i2cdev.regmap).write_regs(BMP180_REGISTER_TEMP_MSB as usize, &[0x6c, 0xfa]);
//         (&mut i2cdev.regmap).write_regs(BMP180_REGISTER_AC1MSB as usize,
//                                         &[0x1, 0x98 /* ac1 */, 0xff, 0xb8 /* ac2 */, 0xc7, 0xd1 /* ac3 */, 0x7f, 0xe5 /* ac4 */, 0x7f, 0xf5 /* ac5 */, 0x5a, 0x71 /* ac6 */, 0x18, 0x2e /* b1 */, 0x0,
//                                             0x04 /* b2 */, 0x80, 0x0 /* mb */, 0xdd, 0xf9 /* mc */, 0xb, 0x34 /* md */]); // C12
//         (&mut i2cdev.regmap).write_regs(BMP180_REGISTER_PRESSURE_MSB_TEST, &[0x5d, 0x23, 0x0]);
//         BMP180BarometerThermometer::new(i2cdev, BMP180PressureMode::BMP180UltraLowPower).unwrap()
//     }

//     #[test]
//     fn test_calculate_real_pressure() {
//         // this hasged code  below will work when BMP180_REGISTER_PRESSURE_MSB = 0x90
//         // to bypass issue related to holding temp and pressuire in the same BMP180 register

//         // let mut i2cdev = MockI2CDevice::new();
//         // let mut bmp180 = make_dev(i2cdev);
//         // println!("test_calculate_real_pressure(): pressure_kpa: {}",
//         //          bmp180.pressure_hpa().unwrap());
//         // Static values from BMP180 datasheet page 15 (Figure 4

//         // mockup for calculate_real_pressure() code test
//         let raw = BMP180RawReading {
//             tadc: 27898,
//             padc: 23843,
//         };
//         let b5 = 2399;
//         // Coefficients from BMP180 documentation for calculating scenario
//         let test_coeff = BMP180CalibrationCoefficients {
//             ac1: 408,
//             ac2: -72,
//             ac3: -14383,
//             ac4: 32741,
//             ac5: 32757,
//             ac6: 23153,
//             b1: 6190,
//             b2: 4,
//             mb: -32768,
//             mc: -8711,
//             md: 2868,
//         };
//         let pressure = calculate_real_pressure(raw.padc,
//                                                b5,
//                                                test_coeff,
//                                                BMP180PressureMode::BMP180UltraLowPower);
//         assert_almost_eq!(pressure, 69964_f32);
//     }

// }