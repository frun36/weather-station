#![no_std]
#![no_main]

use embedded_hal::digital::v2::InputPin;
use rp_pico as bsp;

use bsp::hal::gpio;

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

type LEDPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>;

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

    let button = pins.gpio15.into_pull_down_input();

    display.enable_all();
    delay.delay_ms(500);
    display.disable_all();
    delay.delay_ms(500);

    loop {
        if button.is_high().unwrap() {
            display.display_f8(f8::from_f32(1.23));
            delay.delay_ms(5000);
            display.disable_all();
        }
    }
}
