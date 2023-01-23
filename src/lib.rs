pub mod gpio_pin_data;
// pub mod gpio;
pub mod gpio_new;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    // use crate::gpio::{setmode, setup, output, cleanup, Direction, Level};
    use crate::gpio_new::{GPIO, Direction, Level};

    #[test]
    fn it_works() {
        let mut gpio = GPIO::new();
        gpio.setmode(String::from("BOARD"));

        // ERROR line 344: mutex lock() blocks -> double lock...
        gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW));

        for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                gpio.output(vec![7, 11], vec![Level::HIGH, Level::HIGH]);
                std::thread::sleep(std::time::Duration::from_secs(1));
                gpio.output(vec![7, 11], vec![Level::LOW, Level::LOW]);
            }

            gpio.cleanup(None);
    }
}
