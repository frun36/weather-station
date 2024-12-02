use crate::{
    devices,
    http::{GetAs, GetStr, KeyValueMap, StatusCode},
};
use core::fmt::Write;
use embassy_futures::block_on;
use embassy_rp::rtc::{DateTime, DayOfWeek};
use heapless::Vec;

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

pub fn set_time(content: Option<&KeyValueMap>) -> Result<(), StatusCode> {
    let content = content.ok_or(StatusCode::BadRequest)?;

    let year = content.get_as("y").map_err(|_| StatusCode::BadRequest)?;
    let month = content.get_as("mo").map_err(|_| StatusCode::BadRequest)?;
    let day = content.get_as("d").map_err(|_| StatusCode::BadRequest)?;
    let hour = content.get_as("h").map_err(|_| StatusCode::BadRequest)?;
    let minute = content.get_as("m").map_err(|_| StatusCode::BadRequest)?;
    let day_of_week = match content
        .get_str("day_of_week")
        .map_err(|_| StatusCode::BadRequest)?
    {
        "Mon" => DayOfWeek::Monday,
        "Tue" => DayOfWeek::Tuesday,
        "Wed" => DayOfWeek::Wednesday,
        "Thu" => DayOfWeek::Thursday,
        "Fri" => DayOfWeek::Friday,
        "Sat" => DayOfWeek::Saturday,
        "Sun" => DayOfWeek::Sunday,
        _ => return Err(StatusCode::BadRequest),
    };

    let dt = DateTime {
        year,
        month,
        day,
        hour,
        day_of_week,
        minute,
        second: 0,
    };

    block_on(devices::rtc::set_time(dt)).map_err(|_| StatusCode::InternalServerError)?;

    Ok(())
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
