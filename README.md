# jetson-gpio-rust

A Rust port of NVIDIA's Python library for controlling GPIO pins on select Jetson devices.
It is based on the [jetson-gpio](https://github.com/NVIDIA/jetson-gpio) Python library.

## Getting started

The usage of this crate is very similar to the Python library. The main
difference is that the Python library uses a global `GPIO` object, while this
crate uses a `GPIO` struct. This means that you need to create a `GPIO` struct
before you can use it. The `GPIO` struct is created using the `new` method.

```rust
use jetson_gpio::{GPIO, Direction, Level, Mode};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BOARD).unwrap();

gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW)).unwrap();
gpio.output(vec![7, 11], vec![Level::HIGH, Level::HIGH]).unwrap();

gpio.cleanup(None).unwrap();
```

This example sets up two pins as outputs and sets them to an initial LOW value.
It then sets the pins to HIGH and finally cleans up the GPIO pins. The `unwrap`
method is used to unwrap the `Result` returned by the methods. If an error
occurs, the program will panic (you should handle the error properly in your
code 🤓).

Start using this crate by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
jetson_gpio = { version = "0.1.0" }
```

## Pin numbering mode

Just like in the Python library, you must specify the pin numbering mode
before you can use the GPIO pins. The pin numbering mode can be set using the
`setmode` method. The pin numbering mode can be one of the following:

* `Mode::BOARD` - The pin numbers are the physical pin numbers on the Jetson board.
* `Mode::BCM` - The pin numbers are the Broadcom SOC channel numbers.
* `Mode::TEGRA_SOC` - The pin numbers are the Tegra SOC channel numbers.
* `Mode::CVM` - The pin numbers are the CVM channel numbers.

Using this library, you can configure GPIO pins as either inputs or outputs.
You can also read the current value of an input pin or set the value of an
output pin.

## Crate support

This crate is tested on the following Jetson devices:

* Jetson Xavier NX production module with third-party carrier board (JP4.6.1)

This crate is **under development** and it currently only supports a subset of the
functionality provided by the Python library. Currently supported boards:

* Jetson Orin
* Jetson Xavier NX

Supported pin numbering modes:

* `Mode::BOARD`
* `Mode::BCM`

Only GPIO pins without events are supported.

## License

This crate is licensed under the MIT license. See the [LICENSE](https://github.com/Kajatin/jetson-gpio-rust/blob/main/LICENSE.md) file
for more information.

## Contributing

Contributions are welcome! Please open an issue or a pull request on the [GitHub repository](https://github.com/Kajatin/jetson-gpio-rust)
if you have any questions or suggestions.

---

## TODO

Currently, this crate only supports a subset of the functionality provided by the
Python library. The following features are planned:

* [ ] Add pin definitions for all Jetson boards
* [ ] Add support for all pin modes
* [ ] Add support for PWM pins
* [ ] Test library on all Jetson boards
