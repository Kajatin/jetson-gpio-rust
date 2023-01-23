pub mod gpio;
pub mod gpio_pin_data;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::{gpio::{GPIO, Direction, Level}, gpio_pin_data::Mode};

    #[test]
    fn it_works() {
        let mut gpio = GPIO::new();
        gpio.setmode(Mode::BOARD).unwrap();

        // ERROR line 344: mutex lock() blocks -> double lock...
        gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW)).unwrap();

        for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                gpio.output(vec![7, 11], vec![Level::HIGH, Level::HIGH]).unwrap();
                std::thread::sleep(std::time::Duration::from_secs(1));
                gpio.output(vec![7, 11], vec![Level::LOW, Level::LOW]).unwrap();
            }

            gpio.cleanup(None).unwrap();
    }
}
