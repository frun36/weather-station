#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use dht11::{Dht11, Measurement};
use embedded_hal::digital::v2::InputPin;
use rp_pico as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, InOutPin, Pin, PullDown, SioOutput};

use bsp::entry;
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use defmt_rtt as _;
use panic_probe as _;

use mini_float::f8;

use display::Display;

type LEDPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;

mod dht11;
mod display;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut display = Display::new(
        pins.gpio2.into_push_pull_output().into_dyn_pin(),
        pins.gpio3.into_push_pull_output().into_dyn_pin(),
        pins.gpio4.into_push_pull_output().into_dyn_pin(),
        pins.gpio5.into_push_pull_output().into_dyn_pin(),
        pins.gpio6.into_push_pull_output().into_dyn_pin(),
        pins.gpio7.into_push_pull_output().into_dyn_pin(),
        pins.gpio8.into_push_pull_output().into_dyn_pin(),
        pins.gpio9.into_push_pull_output().into_dyn_pin(),
    );

    display.enable_all();
    delay.delay_ms(1000);
    display.disable_all();
    delay.delay_ms(1000);

    let button = pins.gpio15.into_pull_down_input();

    let dht11_pin = InOutPin::new(pins.gpio16);

    let mut dht11 = Dht11::new(dht11_pin);

    loop {
        if button.is_high().unwrap() {
            display.disable_all();
            delay.delay_ms(1000);
            let measurement = dht11.perform_measurement(&mut delay).unwrap_or_else(|e| {
                match_error(e, &mut display, &mut delay);
                Measurement {
                    temperature: 0,
                    humidity: 0,
                }
            });

            let temperature = measurement.temperature as f32 / 10_f32;

            display.display_f8(f8::from_f32(temperature));
        }
    }
}

fn match_error<E>(e: dht11::Error<E>, display: &mut Display, delay: &mut Delay) {
    let error_code: u8 = match e {
        dht11::Error::Timeout => 0x81,
        dht11::Error::CrcMismatch => 0x82,
        dht11::Error::Gpio(_) => 0x84,
    };
    for _ in 0..3 {
        display.display_u8(error_code);
        delay.delay_ms(200);
        display.disable_all();
        delay.delay_ms(200);
    }
}
