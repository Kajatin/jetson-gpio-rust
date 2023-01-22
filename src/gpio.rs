use std::{fs, sync::Mutex, collections::HashMap, path::Path, io::{Write, Seek, Read}, thread, time::Duration};
use once_cell::sync::Lazy;

use crate::gpio_pin_data::{get_data, JetsonInfo, ChannelInfo};

static SYSFS_ROOT: &str = "/sys/class/gpio";

// Pin numbering modes
static BOARD: i32 = 10;
static BCM: i32 = 11;
static TEGRA_SOC: i32 = 1000;
static CVM: i32 = 1001;

// The constants and their offsets are implemented to prevent HIGH from being
// used in place of other variables (ie. HIGH and RISING should not be
// interchangeable)
// Pull up/down options
static _PUD_OFFSET: i32 = 20;
static PUD_OFF: i32 = 0 + _PUD_OFFSET;
static PUD_DOWN: i32 = 1 + _PUD_OFFSET;
static PUD_UP: i32 = 2 + _PUD_OFFSET;

#[derive(PartialEq, Clone)]
pub enum Level {
    LOW = 0,
    HIGH = 1,
}

// Edge possibilities
// These values (with _EDGE_OFFSET subtracted)
static _EDGE_OFFSET: i32 = 30;
static RISING: i32 = 1 + _EDGE_OFFSET;
static FALLING: i32 = 2 + _EDGE_OFFSET;
static BOTH: i32 = 3 + _EDGE_OFFSET;

// GPIO directions. UNKNOWN constant is for gpios that are not yet setup
#[derive(PartialEq, Clone)]
pub enum Direction {
    UNKNOWN = -1,
    OUT = 0,
    IN = 1,
    HARD_PWM = 43,
}

struct GpioState {
    model: String,
    JETSON_INFO: JetsonInfo,
    _channel_data_by_mode: HashMap<String, HashMap<u32, ChannelInfo>>,

    // # Dictionary objects used as lookup tables for pin to linux gpio mapping
    _channel_data: HashMap<u32, ChannelInfo>,

    _gpio_warnings: bool,
    _gpio_mode: Option<String>,
    _channel_configuration: HashMap<u32, Direction>,
}

impl GpioState {
    fn new() -> Self {
        let (model, JETSON_INFO, _channel_data_by_mode) = get_data();

        GpioState {
            model,
            JETSON_INFO: JETSON_INFO,
            _channel_data_by_mode,

            _channel_data: HashMap::new(),

            _gpio_warnings: true,
            _gpio_mode: None,
            _channel_configuration: HashMap::new(),
        }
    }
}

static GPIO_STATE: Lazy<Mutex<GpioState>> = Lazy::new(|| Mutex::new(GpioState::new()));

fn _validate_mode_set() {
    if GPIO_STATE.lock().unwrap()._gpio_mode.is_none() {
        panic!("Please set pin numbering mode using GPIO.setmode(GPIO.BOARD), GPIO.setmode(GPIO.BCM), GPIO.setmode(GPIO.TEGRA_SOC) or GPIO.setmode(GPIO.CVM)");
    }
}

// Function used to enable/disable warnings during setup and cleanup.
// Param -> state is a bool
fn setwarnings(state: bool){
    GPIO_STATE.lock().unwrap()._gpio_warnings = state;
}

// Function used to set the pin mumbering mode. Possible mode values are BOARD,
// BCM, TEGRA_SOC, and CVM
pub fn setmode(mode: String){
    let mut gpio_state = GPIO_STATE.lock().unwrap();
    // check if a different mode has been set
    if gpio_state._gpio_mode.is_some() && Some(mode.clone()) != gpio_state._gpio_mode{
        panic!("A different mode has already been set!");
    }

    // check if mode parameter is valid
    let accepted_modes = vec!["BOARD", "BCM"];
    if !accepted_modes.contains(&mode.as_str()) {
        panic!("An invalid mode was passed to setmode()!");
    }

    // _channel_data = _channel_data_by_mode[accepted_modes[mode]]
    gpio_state._channel_data = gpio_state._channel_data_by_mode.get(&mode).unwrap().clone();
    gpio_state._gpio_mode = Some(mode);
}

// Function used to get the currently set pin numbering mode
pub fn getmode() -> Option<String> {
    GPIO_STATE.lock().unwrap()._gpio_mode.clone()
}

fn check_write_access() -> bool {
    let export_path = format!("{}/export", SYSFS_ROOT);
    let unexport_path = format!("{}/unexport", SYSFS_ROOT);

    let export_metadata = fs::metadata(&export_path).unwrap();
    let unexport_metadata = fs::metadata(&unexport_path).unwrap();

    let export_permissions = export_metadata.permissions();
    let unexport_permissions = unexport_metadata.permissions();

    !export_permissions.readonly() && !unexport_permissions.readonly()
}

fn _channel_to_info_lookup(channel: u32, need_gpio: bool, need_pwm: bool) -> ChannelInfo {
    let gpio_state = GPIO_STATE.lock().unwrap();
    if !gpio_state._channel_data.contains_key(&channel) {
        panic!("Channel {} is invalid", channel);
    }

    let ch_info = gpio_state._channel_data.get(&channel).unwrap().clone();

    if need_gpio && ch_info.gpio_chip_dir == "" {
        panic!("Channel {} is not a GPIO", channel);
    }

    if need_pwm && ch_info.pwm_chip_dir.is_none() {
        panic!("Channel {} is not a PWM", channel);
    }

    ch_info
}

fn _channel_to_info(channel: u32, need_gpio: bool, need_pwm: bool) -> ChannelInfo {
    _validate_mode_set();
    _channel_to_info_lookup(channel, need_gpio, need_pwm)
}

fn _channels_to_infos(channels: Vec<u32>, need_gpio: bool, need_pwm: bool) -> Vec<ChannelInfo> {
    _validate_mode_set();
    let mut ret: Vec<ChannelInfo> = Vec::new();
    for channel in channels {
        ret.push(_channel_to_info_lookup(channel, need_gpio, need_pwm));
    }

    ret
}

fn _sysfs_channel_configuration(ch_info: ChannelInfo) -> Option<Direction> {
    // """Return the current configuration of a channel as reported by sysfs. Any
    // of IN, OUT, PWM, or None may be returned."""

    if ch_info.pwm_chip_dir.is_some() {
        let pwm_dir = format!("{}/pwm{}", ch_info.pwm_chip_dir.unwrap(), ch_info.pwm_id?);
        if Path::new(&pwm_dir).exists() {
            return Some(Direction::HARD_PWM);
        }
    }

    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if !Path::new(&gpio_dir).exists() {
        return None;
    }

    let gpio_direction = fs::read_to_string(format!("{}/direction", gpio_dir)).unwrap();
    if gpio_direction == "in" {
        return Some(Direction::IN);
    } else if gpio_direction == "out" {
        return Some(Direction::OUT);
    }

    None
}


fn _app_channel_configuration(ch_info: ChannelInfo) -> Option<Direction> {
    // """Return the current configuration of a channel as requested by this
    // module in this process. Any of IN, OUT, or None may be returned."""

    match GPIO_STATE.lock().unwrap()._channel_configuration.get(&ch_info.channel) {
        Some(direction) => Some(direction.clone()),
        None => None,
    }
}

fn _export_gpio(ch_info: ChannelInfo) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if !Path::new(&gpio_dir).exists() {
        let mut f_export = fs::OpenOptions::new()
            .write(true)
            .open(format!("{}/export", SYSFS_ROOT))
            .unwrap();
        f_export.write_all(ch_info.global_gpio.to_string().as_bytes()).unwrap();
    }

    while !Path::new(&format!("{}/value", gpio_dir)).exists() {
        thread::sleep(Duration::from_millis(10));
    }
}

fn _unexport_gpio(ch_info: ChannelInfo) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if Path::new(&gpio_dir).exists() {
        let mut f_unexport = fs::OpenOptions::new()
            .write(true)
            .open(format!("{}/unexport", SYSFS_ROOT))
            .unwrap();
        f_unexport.write_all(ch_info.global_gpio.to_string().as_bytes()).unwrap();
    }
}

fn _cleanup_one(ch_info: ChannelInfo) {
    let mut gpio_state = GPIO_STATE.lock().unwrap();
    match gpio_state._channel_configuration.get(&ch_info.channel) {
        Some(direction) => {
            if direction == &Direction::HARD_PWM {
                // _disable_pwm(ch_info);
                // _unexport_pwm(ch_info);
            } else {
                // event::event_cleanup(ch_info.gpio, ch_info.gpio_name);
                _unexport_gpio(ch_info.clone());
            }
        }
        None => {}
    }

    gpio_state._channel_configuration.remove(&ch_info.channel);
}

fn _cleanup_all() {
    let mut gpio_state = GPIO_STATE.lock().unwrap();
    for (channel, _) in gpio_state._channel_configuration.iter() {
        let ch_info = _channel_to_info(*channel, false, false);
        _cleanup_one(ch_info);
    }

    gpio_state._gpio_mode = None;
}

fn _write_direction(ch_info: ChannelInfo, direction: String) {
    let gpio_dir = format!("{}/{}/direction", SYSFS_ROOT, ch_info.global_gpio_name);
    let mut f_direction = fs::OpenOptions::new()
        .write(true)
        .open(gpio_dir)
        .unwrap();
    f_direction.rewind().unwrap();
    f_direction.write_all(direction.as_bytes()).unwrap();
}

fn _write_value(ch_info: ChannelInfo, value: String) {
    let gpio_dir = format!("{}/{}/value", SYSFS_ROOT, ch_info.global_gpio_name);
    let mut f_direction = fs::OpenOptions::new()
        .write(true)
        .open(gpio_dir)
        .unwrap();
    f_direction.rewind().unwrap();
    f_direction.write_all(value.as_bytes()).unwrap();
}

fn _read_value(ch_info: ChannelInfo) -> String {
    let gpio_dir = format!("{}/{}/value", SYSFS_ROOT, ch_info.global_gpio_name);
    let mut f_direction = fs::OpenOptions::new()
        .read(true)
        .open(gpio_dir)
        .unwrap();
    let mut value = String::new();
    f_direction.rewind().unwrap();
    f_direction.read_to_string(&mut value).unwrap();
    value
}

fn _output_one(ch_info: ChannelInfo, value: Level) {
    let value_str = match value {
        Level::HIGH => "1",
        Level::LOW => "0",
    };

    _write_value(ch_info, value_str.to_string());
}

fn _setup_single_out(ch_info: ChannelInfo, initial: Option<Level>) {
    _export_gpio(ch_info.clone());
    _write_direction(ch_info.clone(), "out".to_string());

    if initial.is_some() {
        _output_one(ch_info.clone(), initial.unwrap());
    }

    GPIO_STATE.lock().unwrap()._channel_configuration.insert(ch_info.channel, Direction::OUT);
}

fn _setup_single_in(ch_info: ChannelInfo) {
    _export_gpio(ch_info.clone());
    _write_direction(ch_info.clone(), "in".to_string());

    GPIO_STATE.lock().unwrap()._channel_configuration.insert(ch_info.channel, Direction::IN);
}

pub fn setup(channels: Vec<u32>, direction: Direction, initial: Option<Level>) {
    if !check_write_access() {
        panic!("The current user does not have permissions set to access the library functionalities. Please configure permissions or use the root user to run this");
    } else {
        println!("The current user has permissions set to access the library functionalities.");
    }

    // if pull_up_down in setup.__defaults__:
    //     pull_up_down_explicit = False
    //     pull_up_down = pull_up_down.val
    // else:
    //     pull_up_down_explicit = True

    let ch_infos = _channels_to_infos(channels, true, false);

    // check direction is valid
    if direction != Direction::OUT && direction != Direction::IN {
        panic!("An invalid direction was passed to setup()");
    }

    // // check if pullup/down is used with output
    // if direction == OUT and pull_up_down != PUD_OFF:
    //     raise ValueError("pull_up_down parameter is not valid for outputs")

    // // check if pullup/down value is specified and/or valid
    // if pull_up_down_explicit:
    //     warnings.warn("Jetson.GPIO ignores setup()'s pull_up_down parameter")
    // if (pull_up_down != PUD_OFF and pull_up_down != PUD_UP and
    //         pull_up_down != PUD_DOWN):
    //     raise ValueError("Invalid value for pull_up_down; should be one of"
    //                      "PUD_OFF, PUD_UP or PUD_DOWN")

    let gpio_state = GPIO_STATE.lock().unwrap();
    if gpio_state._gpio_warnings {
        for ch_info in ch_infos.clone() {
            let sysfs_cfg = _sysfs_channel_configuration(ch_info.clone());
            let app_cfg = _app_channel_configuration(ch_info);

            // warn if channel has been setup external to current program
            if app_cfg.is_none() && sysfs_cfg.is_some() {
                println!("This channel is already in use, continuing anyway. Use GPIO.setwarnings(False) to disable warnings");
            }
        }
    }

    // cleanup if the channel is already setup
    for ch_info in ch_infos.clone() {
        // if ch_info.channel in _channel_configuration:
        //     _cleanup_one(ch_info)
        if gpio_state._channel_configuration.contains_key(&ch_info.channel) {
            _cleanup_one(ch_info);
        }
    }

    match direction {
        Direction::OUT => {
            for ch_info in ch_infos.clone() {
                _setup_single_out(ch_info, initial.clone());
            }
        }
        _ => {
            if initial.is_some() {
                panic!("initial parameter is not valid for inputs");
            }
            for ch_info in ch_infos {
                _setup_single_in(ch_info);
            }
        }
    }
}

// Function used to cleanup channels at the end of the program.
// The param channel can be an integer or list/tuple of integers specifying the
// channels to be cleaned up. If no channel is provided, all channels are
// cleaned
pub fn cleanup(channel: Option<Vec<u32>>) {
    let gpio_state = GPIO_STATE.lock().unwrap();
    // warn if no channel is setup
    if gpio_state._gpio_mode.is_none() {
        if gpio_state._gpio_warnings {
            println!("No channels have been set up yet - nothing to clean up! Try cleaning up at the end of your program instead!");
        }
        return;
    }

    // clean all channels if no channel param provided
    if channel.is_none() {
        _cleanup_all();
        return;
    }

    let ch_infos = _channels_to_infos(channel.unwrap(), false, false);
    for ch_info in ch_infos {
        if gpio_state._channel_configuration.contains_key(&ch_info.channel) {
            _cleanup_one(ch_info);
        }
    }
}

// Function used to return the current value of the specified channel.
// Function returns either HIGH or LOW
pub fn input(channel: u32) -> Level {
    let ch_info = _channel_to_info(channel, true, false);

    let app_cfg = _app_channel_configuration(ch_info.clone());
    if app_cfg.is_none() || ![Direction::IN, Direction::OUT].contains(&app_cfg.unwrap()) {
        panic!("You must setup() the GPIO channel first");
    }

    match _read_value(ch_info).as_str() {
        "0" => Level::LOW,
        _ => Level::HIGH,
    }
}

// Function used to set a value to a channel or list/tuple of channels.
// Parameter channels must be an integer or list/tuple of integers.
// Values must be either HIGH or LOW or list/tuple
// of HIGH and LOW with the same length as the channels list/tuple
pub fn output(channels: Vec<u32>, values: Vec<Level>) {
    let ch_infos = _channels_to_infos(channels, true, false);

    if values.len() != ch_infos.len() {
        panic!("Number of values != number of channels");
    }

    // check that channels have been set as output
    for ch_info in ch_infos.clone() {
        let app_cfg = _app_channel_configuration(ch_info);
        if app_cfg.is_none() || app_cfg.unwrap() != Direction::OUT {
            panic!("The GPIO channel has not been set up as an OUTPUT");
        }
    }

    for (ch_info, value) in ch_infos.iter().zip(values.iter()) {
        _output_one(ch_info.clone(), value.clone());
    }
}
