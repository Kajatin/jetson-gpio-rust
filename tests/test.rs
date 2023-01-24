use jetson_gpio::{GPIO, Direction, Level, Mode};


#[test]
fn test_flash_leds_pin_7() {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD).unwrap();
    gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW)).unwrap();

    for _ in 0..2 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        gpio.output(vec![7, 11], vec![Level::HIGH, Level::HIGH]).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        gpio.output(vec![7, 11], vec![Level::LOW, Level::LOW]).unwrap();
    }

    gpio.cleanup(None).unwrap();
}
