use core::convert::Infallible;

use cortex_m::delay::Delay;
use embedded_hal::digital::v2::InputPin;
use rp_pico as bsp;

use bsp::hal::{gpio::{DynPinId, FunctionSio, Pin, PullDown}, Timer};

pub struct Button {
    button_pin: Pin<DynPinId, FunctionSio<bsp::hal::gpio::SioInput>, PullDown>,
}

impl Button {
    /// Creates new `Button` instance
    pub fn new(button_pin: Pin<DynPinId, FunctionSio<bsp::hal::gpio::SioInput>, PullDown>) -> Self {
        Button { button_pin }
    }

    #[inline]
    /// Checks whether a `Button` is currently clicked
    pub fn is_clicked(&self) -> Result<bool, Infallible> {
        self.button_pin.is_high()
    }

    /// Waits for next click for a short time (e.g. to implement double-click functionality). 
    /// Returns `true` if button was clicked in that time and `false` otherwise
    pub fn await_next_click(&self, delay: &mut Delay, timer: &Timer) -> Result<bool, Infallible> {
        // Wait for the button to be released
        delay.delay_ms(2);
        while self.is_clicked()? {}

        // Wait for the button to be clicked again
        let instant = timer.get_counter_low();
        while timer.get_counter_low() - instant < 500_000 {
            delay.delay_ms(100);
            if self.is_clicked()? {
                return Ok(true);
            }
        }

        // Timeout occured
        Ok(false)
    }
}
