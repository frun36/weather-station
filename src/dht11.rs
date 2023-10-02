use embedded_hal::digital::v2::{InputPin, OutputPin};

use cortex_m::delay::Delay;

use bsp::hal::gpio::{self, InOutPin, Pin, PullDown};

use rp_pico as bsp;

/// How long to wait for a pulse on the data line (in microseconds).
const TIMEOUT_US: u16 = 1_000;

/// DHT11 Error type
#[derive(Debug)]
pub enum Error {
    /// Timeout during communication.
    Timeout,
    /// CRC mismatch.
    CrcMismatch,
    /// GPIO error.
    Gpio(gpio::Error),
}

/// The pin type required for the DHT11 data pin
pub type DHT11Pin =
    InOutPin<Pin<bsp::hal::gpio::bank0::Gpio16, bsp::hal::gpio::FunctionNull, PullDown>>;

/// A DHT11 device.
pub struct Dht11 {
    data_pin: DHT11Pin,
}

/// Results of a reading performed by the DHT11.
#[derive(Copy, Clone, Default, Debug)]
pub struct Measurement {
    /// The measured temperature in tenths of degrees Celsius.
    pub temperature: i16,
    /// The measured humidity in tenths of a percent.
    pub humidity: u16,
}

impl Dht11 {
    /// Creates a new DHT11 device connected to the specified pin.
    pub fn new(data_pin: DHT11Pin) -> Self {
        Dht11 { data_pin }
    }

    /// Performs a reading of the sensor.
    pub fn perform_measurement(&mut self, delay: &mut Delay) -> Result<Measurement, Error> {
        let mut data = [0u8; 5];

        // Perform initial handshake
        self.perform_handshake(delay)?;

        // Read bits
        for i in 0..40 {
            data[i / 8] <<= 1;
            if self.read_bit(delay)? {
                data[i / 8] |= 1;
            }
        }

        // Finally wait for line to go idle again.
        self.wait_for_pulse(true, delay)?;

        // Check CRC
        let crc = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);
        if crc != data[4] {
            return Err(Error::CrcMismatch);
        }

        // Compute temperature
        let mut temp = i16::from(data[2] & 0x7f) * 10 + i16::from(data[3]);
        if data[2] & 0x80 != 0 {
            temp = -temp;
        }

        Ok(Measurement {
            temperature: temp,
            humidity: u16::from(data[0]) * 10 + u16::from(data[1]),
        })
    }

    fn perform_handshake(&mut self, delay: &mut Delay) -> Result<(), Error> {
        // Set pin as floating to let pull-up raise the line and start the reading process.
        self.set_input()?;
        delay.delay_ms(1);

        // Pull line low for at least 18ms to send a start command.
        self.set_low()?;
        delay.delay_ms(20);

        // Restore floating
        self.set_input()?;
        delay.delay_us(40);

        // As a response, the device pulls the line low for 80us and then high for 80us.
        self.read_bit(delay)?;

        Ok(())
    }

    fn read_bit(&mut self, delay: &mut Delay) -> Result<bool, Error> {
        let low = self.wait_for_pulse(true, delay)?;
        let high = self.wait_for_pulse(false, delay)?;
        Ok(high > low)
    }

    fn wait_for_pulse(&mut self, level: bool, delay: &mut Delay) -> Result<u32, Error> {
        let mut count = 0;

        while self.read_line()? != level {
            count += 1;
            if count > TIMEOUT_US {
                return Err(Error::Timeout);
            }
            delay.delay_us(1);
        }
        return Ok(u32::from(count));
    }

    fn set_input(&mut self) -> Result<(), Error> {
        self.data_pin.set_high().map_err(Error::Gpio)
    }

    fn set_low(&mut self) -> Result<(), Error> {
        self.data_pin.set_low().map_err(Error::Gpio)
    }

    fn read_line(&self) -> Result<bool, Error> {
        self.data_pin.is_high().map_err(Error::Gpio)
    }
}
