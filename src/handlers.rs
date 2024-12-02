use embassy_futures::block_on;
use core::fmt::Write;
use heapless::Vec;
use crate::devices;

pub const INDEX: &str = include_str!("../static/index.html");

pub fn write_time<const BUF_SIZE: usize>(buffer: &mut Vec<u8, BUF_SIZE>) {
    let now = block_on(devices::rtc::now());
    if let Some(dt) = now {
        core::write!(
            buffer,
            "{}-{}-{} {:02}:{:02}:{:02}",
            dt.year,
            dt.month,
            dt.day,
            dt.hour,
            dt.minute,
            dt.second
        )
        .unwrap();
    }
}

pub fn write_temperature<const BUF_SIZE: usize>(buffer: &mut Vec<u8, BUF_SIZE>) {
    let reading = block_on(devices::dht::read());
    if let Some(reading) = reading {
        core::write!(
            buffer,
            "T: {} Rh: {}",
            reading.get_temp(),
            reading.get_hum()
        )
        .unwrap();
    }
}