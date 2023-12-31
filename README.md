# Raspberry Pi Pico W weather station
This is the code used to control my simple Raspberry Pi Pico W weather station (shown below displaying a temperature of 23 degrees Celsius).

![Weather station](./img/weather-station-01.jpg "Weather station")

## How it works
The station is very simple to use - all you need to do is plug in the USB power and wait for the LEDs to flash (allows you to check whether there are any dead ones), and then press the little button on the left to ask for the temperature reading. You will be greeted with an opening animation, followed by 10s of displaying the temperature as an eight bit float (`f8`, see https://frun36.github.io/mini-float/), 10s of displaying the relative humidity (`f8`, as a fraction instead of percentage) and the closing animation. Should anything go wrong during the measurement process, the orange LED will flash, along with one of the greed LEDs to show the error code. You will then see no closing animation, and will have to ask for the measurement again. 

## Further development
Currently I'm working on getting the Pico to connect to some other device to store the measurements it has performed, to be able to analyze the temperature fluctuations in my room. This, however, has proved to be very troublesome.

First, I wanted to store the measurements in a file somewhere on the Pico, to then be able to move the file to a different device. To achieve this, I wanted to use `littlefs`. I had some problems compiling the `littlefs2` crate, which took me a good couple of hours to solve. I then realized I have no idea how to initialize the storage, and the documentation wasn't particularly helpful in this regard. I have therefore abandoned the filesystem idea, since I couldn't find any reasonable alternatives to `littlefs`. 

I then decided to connect the Pico to my home WiFi network, and send the data to a server I would run on my Raspberry Pi Zero W. I searched for crates that would help me do this, and maybe even found some, but they were either too complicated for this use-case (`embassy-net`), or provided no documentation whatsoever on how to interface them with the Pico (`smoltcp`). I even tried using the Pico C/C++ SDK and interfacing it with my Rust code, but to no avail - there's just too much of everything and I'm a bit lost.

There's one more thing I'd like to try - UART communication. I got the idea from a tutorial on the C/C++ SDK, and hopefully I'll be able to make it work.
