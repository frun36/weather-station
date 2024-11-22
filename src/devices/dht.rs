use embassy_dht::dht11::DHT11;
use embassy_rp::gpio::AnyPin;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Delay;

static DHT: Mutex<ThreadModeRawMutex, Option<DHT11<'_, Delay>>> = Mutex::new(None);

pub async fn init(pin: AnyPin) {
    let dht = DHT11::new(pin, Delay);

    *DHT.lock().await = Some(dht);
}

pub async fn read() -> Option<embassy_dht::Reading<i8, u8>> {
    let mut dht = DHT.lock().await;

    match dht.as_mut() {
        Some(dht) => dht.read().ok(),
        None => None,
    }
}
