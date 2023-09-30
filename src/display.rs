use crate::LEDPin;

use embedded_hal::digital::v2::OutputPin;

use mini_float::f8;

pub struct Display {
    pins: [LEDPin; 8],
}

impl Display {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sgn: LEDPin,
        exp2: LEDPin,
        exp1: LEDPin,
        exp0: LEDPin,
        man1: LEDPin,
        man2: LEDPin,
        man3: LEDPin,
        man4: LEDPin,
    ) -> Self {
        Self {
            pins: [man4, man3, man2, man1, exp0, exp1, exp2, sgn],
        }
    }

    pub fn enable_all(&mut self) {
        for pin in &mut self.pins {
            pin.set_high().unwrap();
        }
    }

    pub fn disable_all(&mut self) {
        for pin in &mut self.pins {
            pin.set_low().unwrap();
        }
    }

    pub fn display_u8(&mut self, x: u8) {
        for i in 0..8 {
            if (x & (1 << (7 - i))) >> (7 - i) == 1 {
                self.pins[i].set_high().unwrap();
            } else {
                self.pins[i].set_low().unwrap();
            }
        }
    }

    pub fn display_f8(&mut self, x: f8) {
        self.display_u8(x.as_byte());
    }
}