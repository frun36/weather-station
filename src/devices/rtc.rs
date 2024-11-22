use defmt::*;
use embassy_rp::{
    peripherals::RTC,
    rtc::{DateTime, DayOfWeek, Rtc},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

static RTC: Mutex<ThreadModeRawMutex, Option<Rtc<'_, RTC>>> = Mutex::new(None);

pub async fn init(rtc: RTC) {
    let mut rtc = Rtc::new(rtc);
    if !rtc.is_running() {
        info!("Start RTC");
        let now = DateTime {
            year: 2024,
            month: 11,
            day: 23,
            day_of_week: DayOfWeek::Saturday,
            hour: 0,
            minute: 0,
            second: 0,
        };
        rtc.set_datetime(now).unwrap();
    }

    *RTC.lock().await = Some(rtc);
}

pub async fn now() -> Option<DateTime> {
    let mut rtc = RTC.lock().await;

    rtc.as_mut().map(|rtc| rtc.now().unwrap())
}
