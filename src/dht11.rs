use embedded_hal::{
    blocking::delay::{DelayMs, DelayUs},
    digital::v2::{InputPin, OutputPin},
};

/// How long to wait for a pulse on the data line (in microseconds).
const TIMEOUT_US: u16 = 1_000;

pub struct Dht11<GPIO> {
    data_pin: GPIO,
}

#[derive(Debug)]
pub enum Error<E> {
    /// Timeout during communication.
    Timeout,
    /// CRC mismatch.
    CrcMismatch,
    /// GPIO error.
    Gpio(E),
}

/// Results of a reading performed by the DHT11.
#[derive(Copy, Clone, Default, Debug)]
pub struct Measurement {
    /// The measured temperature in tenths of degrees Celsius.
    pub temperature: i16,
    /// The measured humidity in tenths of a percent.
    pub humidity: u16,
}

impl<GPIO, E> Dht11<GPIO>
where
    GPIO: InputPin<Error = E> + OutputPin<Error = E>,
{
    pub fn new(data_pin: GPIO) -> Self {
        Dht11 { data_pin }
    }

    pub fn perform_measurement<D>(&mut self, delay: &mut D) -> Result<Measurement, Error<E>>
    where
        D: DelayUs<u16> + DelayMs<u16>,
    {
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

    fn perform_handshake<D>(&mut self, delay: &mut D) -> Result<(), Error<E>>
    where
        D: DelayUs<u16> + DelayMs<u16>,
    {
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

    fn set_input(&mut self) -> Result<(), Error<E>> {
        self.data_pin.set_high().map_err(Error::Gpio)
    }

    fn set_low(&mut self) -> Result<(), Error<E>> {
        self.data_pin.set_low().map_err(Error::Gpio)
    }

    fn read_line(&self) -> Result<bool, Error<E>> {
        self.data_pin.is_high().map_err(Error::Gpio)
    }

    fn read_bit<D>(&mut self, delay: &mut D) -> Result<bool, Error<E>>
    where
        D: DelayUs<u16> + DelayMs<u16>,
    {
        let low = self.wait_for_pulse(true, delay)?;
        let high = self.wait_for_pulse(false, delay)?;
        Ok(high > low)
    }

    fn wait_for_pulse<D>(&mut self, level: bool, delay: &mut D) -> Result<u32, Error<E>>
    where
        D: DelayUs<u16> + DelayMs<u16>,
    {
        let mut count = 0;

        #[cfg(feature = "dwt")]
        let start = DWT::get_cycle_count();

        while self.read_line()? != level {
            count += 1;
            if count > TIMEOUT_US {
                return Err(Error::Timeout);
            }
            delay.delay_us(1);
        }

        #[cfg(feature = "dwt")]
        return Ok(DWT::get_cycle_count().wrapping_sub(start));

        #[cfg(not(feature = "dwt"))]
        return Ok(u32::from(count));
    }
}
