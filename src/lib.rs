//! # Jetson GPIO
//!
//! This crate provides a safe interface to the GPIO pins on the Jetson devices.
//! It is based on the [jetson-gpio](https://github.com/NVIDIA/jetson-gpio) Python
//! library.
//!
//! ```rust
//! use jetson_gpio::{GPIO, Direction, Level, Mode};
//!
//! let mut gpio = GPIO::new();
//! gpio.setmode(Mode::BOARD).unwrap();
//!
//! gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW)).unwrap();
//! gpio.output(vec![7, 11], vec![Level::HIGH, Level::HIGH]).unwrap();
//!
//! gpio.cleanup(None).unwrap();
//! ```
//!
//! There are four different pin numbering modes supported by the Jetson GPIO:
//!
//! * `BOARD` - The pin numbers are the physical pin numbers on the Jetson board.
//! * `BCM` - The pin numbers are the Broadcom SOC channel numbers.
//! * `TEGRA_SOC` - The pin numbers are the Tegra SOC channel numbers.
//! * `CVM` - The pin numbers are the CVM channel numbers.
//!
//! Using this library, you can configure GPIO pins as either inputs or outputs.
//! You can also read the current value of an input pin or set the value of an
//! output pin.
//!
//! # Crate support
//!
//! You can use this crate in your project by adding the following to your
//! `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! jetson_gpio = { version = "0.1.0" }
//! ```

// reexport the gpio module
mod gpio;
mod gpio_pin_data;
pub use gpio::*;
pub use gpio_pin_data::*;
