// Copyright 2018 The Open Cisterna project developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate serialport;

use maxsonar::serialport::prelude::*;
use std::io;
use std::str;
use std::thread;
use std::time::Duration;
use sysfs_gpio::{Direction, Error, Pin};

// GPIO connected to port 4 of the MaxSonar sensor. BCM 22 is located on physical pin 15.
const GPIO_TRIGGER: u64 = 22;
const FREE_RUN_MODE_DELAY_MILLIS: u64 = 132; // MB7380 datasheet page 9

pub fn read_distance(port_name: &str) -> Result<u16, String>  {
    let trigger = Pin::new(GPIO_TRIGGER);
    let init = || -> Result<(), Error> {
        trigger.export()?;
        trigger.set_direction(Direction::Out)?;
        trigger.set_value(1)?;
        thread::sleep(Duration::from_millis((FREE_RUN_MODE_DELAY_MILLIS as f64 * 1.1) as u64));
        Ok(())
    };
    try!(init().map_err(|e| e.to_string()));
    let result = read_from_serial(port_name)?;
    let deinit = || -> Result<(), Error> {
        trigger.set_value(0)?;
        Ok(())
    };
    try!(deinit().map_err(|e| e.to_string()));
    Ok(result)
}

fn read_from_serial(port_name: &str) -> Result<u16, String> {
    let mut settings: SerialPortSettings = Default::default();
    settings.timeout = Duration::from_millis(100);
    if let Ok(mut port) = serialport::open_with_settings(&port_name, &settings) {
        let mut serial_buf: Vec<u8> = vec![0; 6];
        let mut v: Vec<u8> = Vec::with_capacity(5);
        loop {
            let read = port.read(serial_buf.as_mut_slice()).map(|r| r as usize);
            match read {
                Ok(t) => {
                    debug!("Read {} bytes from serial port '{}': {:?}", t, port_name, serial_buf);
                    match v.len() {
                        0 => v.extend(
                                serial_buf[..t]
                                    .iter()
                                    .skip_while(|b| **b != 82u8)
                                    .take_while(|b| **b != 13u8)),
                        _ => v.extend(
                                serial_buf[..t]
                                    .iter()
                                    .take_while(|b| **b != 13u8))
                    }
                    debug!("Read buffer contents: {:?}", v);
                    if v.len() == 5 {
                        let r = match str::from_utf8(&v[..5]) {
                            Ok(v) => {
                                let stripped: String = v.chars().skip(1).collect();
                                stripped.parse::<u16>().or_else(|e| Err(e.to_string()))
                            },
                            Err(e) => Err(format!("Invalid UTF-8 sequence: {}", e))
                        };
                        v.clear();
                        return r
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut =>
                    error!("Timed out while reading from serial port '{}'", port_name),
                Err(e) =>
                    error!("Unexpected error encountered while reading from serial port '{}': {:?}", port_name, e),
            }
        }
    } else {
        Err(format!("Port '{}' not available", &port_name))
    }
}
