pub mod gpio_pin_data;
pub mod gpio;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::gpio::{setmode, setup, output, cleanup, Direction, Level};

    #[test]
    fn it_works() {
        setmode(String::from("BOARD"));

        // ERROR line 344: mutex lock() blocks -> double lock...
        setup(vec![7, 11], Direction::OUT, Some(Level::LOW));

        for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                output(vec![7, 11], vec![Level::HIGH, Level::HIGH]);
                std::thread::sleep(std::time::Duration::from_secs(1));
                output(vec![7, 11], vec![Level::LOW, Level::LOW]);
            }

            cleanup(None);
    }
}
