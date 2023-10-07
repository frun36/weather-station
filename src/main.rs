#![no_std]
#![no_main]

use cortex_m::delay::Delay;

use bsp::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{DynPinId, FunctionSio, InOutPin, Pin, PullDown, SioOutput},
        pac,
        sio::Sio,
        watchdog::Watchdog,
        Timer,
    },
};
use rp_pico as bsp;

use defmt_rtt as _;
use panic_probe as _;

use mini_float::f8;

use button::Button;
use dht11::{DHT11Pin, Dht11, Measurement};
use display::Display;

type LEDPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;

mod button;
mod dht11;
mod display;

enum Command {
    Measure,
    Fun,
}

#[entry]
fn main() -> ! {
    // Initialize basic peripherals
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Initialize clocks, delay and timer
    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
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

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // Initialize devices
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

    let button = Button::new(pins.gpio15.into_pull_down_input().into_dyn_pin());

    let dht11_pin: DHT11Pin = InOutPin::new(pins.gpio16);
    let mut dht11 = Dht11::new(dht11_pin);

    // Blink display
    display.enable_all();
    delay.delay_ms(1_000);
    display.disable_all();

    // Main loop
    let mut command;
    loop {
        if button.is_clicked().unwrap() {
            command = Command::Measure;
            if button.await_next_click(&mut delay, &timer).unwrap() {
                command = Command::Fun;
            }
            match command {
                Command::Measure => {
                    measurement_cycle(&mut dht11, &mut display, &mut delay);
                    // display.roll_fwd(&mut delay, 100);
                    // display.disable_all();
                }
                Command::Fun => {
                    display.roll_bwd(&mut delay, 100);
                    display.disable_all();
                }
            }
        }
    }
}

/// The whole process of measurement and showing the results
fn measurement_cycle(dht11: &mut Dht11, display: &mut Display, delay: &mut Delay) {
    display.roll_fwd(delay, 128);
    delay.delay_ms(256);
    display.disable_all();

    let measurement = dht11.perform_measurement(delay).unwrap_or_else(|e| {
        match_error(e, display, delay);
        Measurement {
            temperature: 0,
            humidity: 0,
        }
    });

    if measurement.temperature == 0 && measurement.humidity == 0 {
        return;
    }

    let temperature = measurement.temperature as f32 * 0.1_f32; // Convert to Celsius
    let humidity = measurement.humidity as f32 * 0.001_f32; // Convert to fraction

    display.display_f8(f8::from_f32(temperature));
    delay.delay_ms(10_000);
    display.disable_all();

    delay.delay_ms(200);

    display.display_f8(f8::from_f32(humidity));
    delay.delay_ms(10_000);
    display.disable_all();

    display.roll_bwd(delay, 128);
    delay.delay_ms(256);
    display.disable_all();
}

/// Displays correct error code
fn match_error(e: dht11::Error, display: &mut Display, delay: &mut Delay) {
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
