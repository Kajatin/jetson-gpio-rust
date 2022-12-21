pub mod gpio_pin_data;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::gpio_pin_data::get_data;

    #[test]
    fn it_works() {
        let ret = get_data();
        println!("ret: {:?}", ret);

        let result: usize = 4;
        assert_eq!(result, 4);
    }
}
