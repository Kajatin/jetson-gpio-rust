use anyhow::Result;
use anyhow::anyhow;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    path::Path,
};

/// Specifies the pin numbering mode.
///
/// The pin numbering mode is used to determine the mapping between the pin numbers
/// and the GPIO channels. The pin numbering mode can be one of the following:
///
/// * `BOARD` - The pin numbers are the physical pin numbers on the Jetson board.
/// * `BCM` - The pin numbers are the Broadcom SOC channel numbers.
/// * `TEGRA_SOC` - The pin numbers are the Tegra SOC channel numbers.
/// * `CVM` - The pin numbers are the CVM channel numbers.
///
/// # Example
///
/// ```rust
/// use jetson_gpio::{GPIO, Mode};
///
/// let mut gpio = GPIO::new();
/// gpio.setmode(Mode::BOARD).unwrap();
/// ```
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub enum Mode {
    BOARD,
    BCM,
    // TEGRA_SOC,
    // CVM,
}

impl Mode {
    /// Converts a string to a `Mode` enum.
    ///
    /// Valid strings are:
    ///
    /// * `"BOARD"`
    /// * `"BCM"`
    /// * `"TEGRA_SOC"`
    /// * `CVM`
    ///
    /// # Example
    ///
    /// ```rust
    /// use jetson_gpio::{GPIO, Mode};
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::from_str("BOARD").unwrap()).unwrap();
    /// ```
    pub fn from_str(s: &str) -> Result<Mode> {
        match s {
            "BOARD" => Ok(Mode::BOARD),
            "BCM" => Ok(Mode::BCM),
            // "TEGRA_SOC" => Ok(Mode::TEGRA_SOC),
            // "CVM" => Ok(Mode::CVM),
            _ => Err(anyhow!("Invalid mode: {}", s)),
        }
    }

    /// Converts a `Mode` enum to a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use jetson_gpio::{GPIO, Mode};
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// assert_eq!(gpio.getmode().unwrap().to_str(), "BOARD");
    /// ```
    pub fn to_str(&self) -> &str {
        match self {
            Mode::BOARD => "BOARD",
            Mode::BCM => "BCM",
            // Mode::TEGRA_SOC => "TEGRA_SOC",
            // Mode::CVM => "CVM",
        }
    }

    /// Checks if the `Mode` is valid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use jetson_gpio::{GPIO, Mode};
    ///
    /// let mut gpio = GPIO::new();
    /// assert_eq!(Mode::BOARD.is_valid(), true);
    /// assert_eq!(Mode::BCM.is_valid(), true);
    /// assert_eq!(Mode::TEGRA_SOC.is_valid(), false);
    /// assert_eq!(Mode::CVM.is_valid(), false);
    /// ```
    pub fn is_valid(&self) -> bool {
        match self {
            Mode::BOARD => true,
            Mode::BCM => true,
            // Mode::TEGRA_SOC => true,
            // Mode::CVM => true,
            _ => false,
        }
    }
}

static CLARA_AGX_XAVIER: &str = "CLARA_AGX_XAVIER";
static JETSON_NX: &str = "JETSON_NX";
static JETSON_XAVIER: &str = "JETSON_XAVIER";
static JETSON_TX2: &str = "JETSON_TX2";
static JETSON_TX1: &str = "JETSON_TX1";
static JETSON_NANO: &str = "JETSON_NANO";
static JETSON_TX2_NX: &str = "JETSON_TX2_NX";
static JETSON_ORIN: &str = "JETSON_ORIN";

static JETSON_MODELS: [&str; 8] = [
    CLARA_AGX_XAVIER,
    JETSON_NX,
    JETSON_XAVIER,
    JETSON_TX2,
    JETSON_TX1,
    JETSON_NANO,
    JETSON_TX2_NX,
    JETSON_ORIN,
];

/// Contains all relevant GPIO data for each Jetson platform.
///
/// This information is automatically configured during the initialization of the library.
/// The fields are:
/// - Linux GPIO pin number (within chip, not global)
/// - Linux exported GPIO name, (entries omitted if exported filename is gpio%i)
/// - GPIO chip sysfs directory
/// - Pin number (BOARD mode)
/// - Pin number (BCM mode)
/// - Pin name (CVM mode)
/// - Pin name (TEGRA_SOC mode)
/// - PWM chip sysfs directory
/// - PWM ID within PWM chip
#[derive(Clone, Debug)]
struct PinDefinition {
    gpio: HashMap<u32, u32>,
    name: HashMap<u32, String>,
    chip_sysfs: String,
    board: u32,
    bcm: u32,
    cvm: String,
    tegra_soc: String,
    pwm_chip_sysfs: Option<String>,
    pwm_id: Option<u32>,
}

/// Contains information about a single GPIO channel.
///
/// This information is automatically gathered during the initialization of the library.
/// The fields are:
/// * `channel`: Channel number
/// * `gpio_chip_dir`: GPIO chip sysfs directory
/// * `gpio`: Linux GPIO pin number (within chip, not global)
/// * `global_gpio`: Linux exported GPIO number (global)
/// * `global_gpio_name`: Linux exported GPIO name
/// * `pwm_chip_dir`: PWM chip sysfs directory
/// * `pwm_id`: PWM ID within PWM chip
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel: u32,
    pub gpio_chip_dir: String,
    pub gpio: HashMap<u32, u32>,
    pub global_gpio: u32,
    pub global_gpio_name: String,
    pub pwm_chip_dir: Option<String>,
    pub pwm_id: Option<u32>,
}

/// Contains information about the Jetson platform.
///
/// This information is automatically gathered during the initialization of the library.
/// The fields are:
/// * `p1_revision`: P1 revision number
/// * `ram`: RAM size
/// * `revision`: Board revision
/// * `ttype`: Board type
/// * `manufacturer`: Board manufacturer
/// * `processor`: Processor type
#[derive(Debug, Clone)]
pub struct JetsonInfo {
    pub p1_revision: u32,
    pub ram: String,
    pub revision: String,
    pub ttype: String,
    pub manufacturer: String,
    pub processor: String,
}

fn read_file_to_string(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents.trim().to_string(),
        Err(e) => {
            // error!("Failed to read file {}: {}", path, e);
            String::from("")
        }
    }
}

fn string_to_uint(s: String) -> u32 {
    match s.parse::<u32>() {
        Ok(i) => i,
        Err(e) => {
            // error!("Failed to parse string to unsigned integer: {}", e);
            0
        }
    }
}

fn find_pmgr_board(prefix: &str) -> Option<String> {
    let ids_path = "/proc/device-tree/chosen/plugin-manager/ids";
    let ids_path_k510 = "/proc/device-tree/chosen/ids";

    if Path::new(ids_path).exists() {
        for f in Path::new(ids_path).read_dir().unwrap() {
            let f = f.unwrap();
            let f = f.file_name().into_string().unwrap();
            if f.starts_with(prefix) {
                return Some(f);
            }
        }
    } else if Path::new(ids_path_k510).exists() {
        let mut f = File::open(ids_path_k510).unwrap();
        let mut ids = String::new();
        f.read_to_string(&mut ids).unwrap();
        for s in ids.split_whitespace() {
            if s.starts_with(prefix) {
                return Some(s.to_string());
            }
        }
    } else {
        eprintln!("WARNING: Plugin manager information missing from device tree.");
        eprintln!("WARNING: Cannot determine whether the expected Jetson board is present.");
    }

    None
}

fn warn_if_not_carrier_board(carrier_boards: &[&str]) {
    let mut found = false;
    for b in carrier_boards {
        found = find_pmgr_board(format!("{}-", b).as_str()).is_some();
        if found {
            break;
        }
    }

    if !found {
        eprintln!("WARNING: Carrier board is not from a Jetson Developer Kit.");
        eprintln!("WARNNIG: This library has not been verified with this carrier board,");
        eprintln!("WARNING: and in fact is unlikely to work correctly.");
    }
}

fn get_model() -> Result<String> {
    let compatible_path = "/proc/device-tree/compatible";

    let compats_jetson_orins = [
        "nvidia,p3737-0000+p3701-0000",
        "nvidia,p3737-0000+p3701-0004",
    ];

    let compats_clara_agx_xavier = ["nvidia,e3900-0000+p2888-0004"];

    let compats_nx = [
        "nvidia,p3509-0000+p3668-0000",
        "nvidia,p3509-0000+p3668-0001",
        "nvidia,p3449-0000+p3668-0000",
        "nvidia,p3449-0000+p3668-0001",
        "nvidia,p3449-0000+p3668-0003",
    ];

    let compats_xavier = [
        "nvidia,p2972-0000",
        "nvidia,p2972-0006",
        "nvidia,jetson-xavier",
        "nvidia,galen-industrial",
        "nvidia,jetson-xavier-industrial",
    ];

    let compats_tx2_nx = ["nvidia,p3509-0000+p3636-0001"];

    let compats_tx2 = [
        "nvidia,p2771-0000",
        "nvidia,p2771-0888",
        "nvidia,p3489-0000",
        "nvidia,lightning",
        "nvidia,quill",
        "nvidia,storm",
    ];

    let compats_tx1 = ["nvidia,p2371-2180", "nvidia,jetson-cv"];

    let compats_nano = [
        "nvidia,p3450-0000",
        "nvidia,p3450-0002",
        "nvidia,jetson-nano",
    ];

    if Path::new(compatible_path).exists() {
        let mut compats = Vec::new();
        let mut file = File::open(compatible_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        for c in contents.split('\x00') {
            compats.push(c);
        }

        fn matches(vals: &[&str], compats: &Vec<&str>) -> bool {
            for v in vals {
                if compats.contains(v) {
                    return true;
                }
            }
            return false;
        }

        if matches(&compats_jetson_orins, &compats) {
            warn_if_not_carrier_board(&["3737", "0000"]);
            return Ok(String::from(JETSON_ORIN));
        } else if matches(&compats_clara_agx_xavier, &compats) {
            warn_if_not_carrier_board(&["3900"]);
            return Ok(String::from(CLARA_AGX_XAVIER));
        } else if matches(&compats_nx, &compats) {
            warn_if_not_carrier_board(&["3509", "3449"]);
            return Ok(String::from(JETSON_NX));
        } else if matches(&compats_xavier, &compats) {
            warn_if_not_carrier_board(&["2822"]);
            return Ok(String::from(JETSON_XAVIER));
        } else if matches(&compats_tx2_nx, &compats) {
            warn_if_not_carrier_board(&["3509"]);
            return Ok(String::from(JETSON_TX2_NX));
        } else if matches(&compats_tx2, &compats) {
            warn_if_not_carrier_board(&["2597"]);
            return Ok(String::from(JETSON_TX2));
        } else if matches(&compats_tx1, &compats) {
            warn_if_not_carrier_board(&["2597"]);
            return Ok(String::from(JETSON_TX1));
        } else if matches(&compats_nano, &compats) {
            let module_id = find_pmgr_board(&"3448");
            if module_id.is_none() {
                anyhow::bail!("Could not determine Jetson Nano module revision");
            }

            let module_id = module_id.unwrap();
            let revision = module_id.split('-').last().unwrap();
            // Revision is an ordered string, not a decimal integer
            if revision < "200" {
                anyhow::bail!("Jetson Nano module revision must be A02 or later");
            }

            warn_if_not_carrier_board(&["3449", "3542"]);
            return Ok(String::from(JETSON_NANO));
        }
    }

    // get model info from the environment variables for docker containers
    let model_name = env::var("JETSON_MODEL_NAME");
    if model_name.is_ok() {
        let model_name = model_name.unwrap();
        let model_name = model_name.trim();
        if JETSON_MODELS.contains(&model_name) {
            return Ok(String::from(model_name));
        } else {
            eprintln!(
                "Environment variable 'JETSON_MODEL_NAME={}' is invalid.",
                model_name
            );
        }
    }

    // raise Exception('Could not determine Jetson model')
    anyhow::bail!("Could not determine Jetson model");
}

fn get_pin_defs(model: &str) -> Result<Vec<PinDefinition>, anyhow::Error> {
    let jetson_orin_pin_defs = [
        PinDefinition {
            gpio: HashMap::from([(164, 106)]),
            name: HashMap::from([(164, String::from("PQ.06"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 7,
            bcm: 4,
            cvm: String::from("MCLK05"),
            tegra_soc: String::from("GP66"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        // Output-only (due to base board)
        PinDefinition {
            gpio: HashMap::from([(164, 112)]),
            name: HashMap::from([(164, String::from("PR.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 11,
            bcm: 17,
            cvm: String::from("UART1_RTS"),
            tegra_soc: String::from("GP72_UART1_RTS_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 50)]),
            name: HashMap::from([(164, String::from("PH.07"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 12,
            bcm: 18,
            cvm: String::from("I2S2_CLK"),
            tegra_soc: String::from("GP122"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 108)]),
            name: HashMap::from([(164, String::from("PR.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 13,
            bcm: 27,
            cvm: String::from("PWM01"),
            tegra_soc: String::from("GP68"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 85)]),
            name: HashMap::from([(164, String::from("PN.01"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 15,
            bcm: 22,
            cvm: String::from("GPIO27"),
            tegra_soc: String::from("GP88_PWM1"),
            pwm_chip_sysfs: Some(String::from("3280000.pwm")),
            pwm_id: Some(0),
        },
        PinDefinition {
            gpio: HashMap::from([(32, 9)]),
            name: HashMap::from([(32, String::from("PBB.01"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 16,
            bcm: 23,
            cvm: String::from("GPIO08"),
            tegra_soc: String::from("GP26"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 43)]),
            name: HashMap::from([(164, String::from("PH.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 18,
            bcm: 24,
            cvm: String::from("GPIO35"),
            tegra_soc: String::from("GP115"),
            pwm_chip_sysfs: Some(String::from("32c0000.pwm")),
            pwm_id: Some(0),
        },
        PinDefinition {
            gpio: HashMap::from([(164, 135)]),
            name: HashMap::from([(164, String::from("PZ.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 19,
            bcm: 10,
            cvm: String::from("SPI1_MOSI"),
            tegra_soc: String::from("GP49_SPI1_MOSI"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 134)]),
            name: HashMap::from([(164, String::from("PZ.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 21,
            bcm: 9,
            cvm: String::from("SPI1_MISO"),
            tegra_soc: String::from("GP48_SPI1_MISO"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 96)]),
            name: HashMap::from([(164, String::from("PP.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 22,
            bcm: 25,
            cvm: String::from("GPIO17"),
            tegra_soc: String::from("GP56"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 133)]),
            name: HashMap::from([(164, String::from("PZ.03"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 23,
            bcm: 11,
            cvm: String::from("SPI1_CLK"),
            tegra_soc: String::from("GP47_SPI1_CLK"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 136)]),
            name: HashMap::from([(164, String::from("PZ.06"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 24,
            bcm: 8,
            cvm: String::from("SPI1_CS0_N"),
            tegra_soc: String::from("GP50_SPI1_CS0_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 137)]),
            name: HashMap::from([(164, String::from("PZ.07"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 26,
            bcm: 7,
            cvm: String::from("SPI1_CS1_N"),
            tegra_soc: String::from("GP51_SPI1_CS1_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(32, 1)]),
            name: HashMap::from([(32, String::from("PAA.01"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 29,
            bcm: 5,
            cvm: String::from("CAN0_DIN"),
            tegra_soc: String::from("GP18_CAN0_DIN"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(32, 0)]),
            name: HashMap::from([(32, String::from("PAA.00"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 31,
            bcm: 6,
            cvm: String::from("CAN0_DOUT"),
            tegra_soc: String::from("GP17_CAN0_DOUT"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(32, 8)]),
            name: HashMap::from([(32, String::from("PBB.00"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 32,
            bcm: 12,
            cvm: String::from("GPIO09"),
            tegra_soc: String::from("GP25"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(32, 2)]),
            name: HashMap::from([(32, String::from("PAA.02"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 33,
            bcm: 13,
            cvm: String::from("CAN1_DOUT"),
            tegra_soc: String::from("GP19_CAN1_DOUT"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 53)]),
            name: HashMap::from([(164, String::from("PI.02"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 35,
            bcm: 19,
            cvm: String::from("I2S2_FS"),
            tegra_soc: String::from("GP125"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 113)]),
            name: HashMap::from([(164, String::from("PR.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 36,
            bcm: 16,
            cvm: String::from("UART1_CTS"),
            tegra_soc: String::from("GP73_UART1_CTS_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(32, 3)]),
            name: HashMap::from([(32, String::from("PAA.03"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 37,
            bcm: 26,
            cvm: String::from("CAN1_DIN"),
            tegra_soc: String::from("GP20_CAN1_DIN"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 52)]),
            name: HashMap::from([(164, String::from("PI.01"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 38,
            bcm: 20,
            cvm: String::from("I2S2_DIN"),
            tegra_soc: String::from("GP124"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(164, 51)]),
            name: HashMap::from([(164, String::from("PI.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 40,
            bcm: 21,
            cvm: String::from("I2S2_DOUT"),
            tegra_soc: String::from("GP123"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
    ];

    let clara_agx_xavier_pin_defs: [PinDefinition; 0] = [];

    let jetson_nx_pin_defs = [
        PinDefinition {
            gpio: HashMap::from([(224, 148), (169, 118)]),
            name: HashMap::from([(169, String::from("PS.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 7,
            bcm: 4,
            cvm: String::from("GPIO09"),
            tegra_soc: String::from("AUD_MCLK"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 140), (169, 112)]),
            name: HashMap::from([(169, String::from("PR.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 11,
            bcm: 17,
            cvm: String::from("UART1_RTS"),
            tegra_soc: String::from("UART1_RTS"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 157), (169, 127)]),
            name: HashMap::from([(169, String::from("PT.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 12,
            bcm: 18,
            cvm: String::from("I2S0_SCLK"),
            tegra_soc: String::from("DAP5_SCLK"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 192), (169, 149)]),
            name: HashMap::from([(169, String::from("PY.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 13,
            bcm: 27,
            cvm: String::from("SPI1_SCK"),
            tegra_soc: String::from("SPI3_SCK"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(40, 20), (30, 16)]),
            name: HashMap::from([(30, String::from("PCC.04"))]),
            chip_sysfs: String::from("c2f0000.gpio"),
            board: 15,
            bcm: 22,
            cvm: String::from("GPIO12"),
            tegra_soc: String::from("TOUCH_CLK"),
            pwm_chip_sysfs: Some(String::from("c340000.pwm")),
            pwm_id: Some(0),
        },
        PinDefinition {
            gpio: HashMap::from([(224, 196), (169, 153)]),
            name: HashMap::from([(169, String::from("PY.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 16,
            bcm: 23,
            cvm: String::from("SPI1_CS1"),
            tegra_soc: String::from("SPI3_CS1_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 195), (169, 152)]),
            name: HashMap::from([(169, String::from("PY.03"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 18,
            bcm: 24,
            cvm: String::from("SPI1_CS0"),
            tegra_soc: String::from("SPI3_CS0_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 205), (169, 162)]),
            name: HashMap::from([(169, String::from("PZ.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 19,
            bcm: 10,
            cvm: String::from("SPI0_MOSI"),
            tegra_soc: String::from("SPI1_MOSI"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 204), (169, 161)]),
            name: HashMap::from([(169, String::from("PZ.04"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 21,
            bcm: 9,
            cvm: String::from("SPI0_MISO"),
            tegra_soc: String::from("SPI1_MISO"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 193), (169, 150)]),
            name: HashMap::from([(169, String::from("PY.01"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 22,
            bcm: 25,
            cvm: String::from("SPI1_MISO"),
            tegra_soc: String::from("SPI3_MISO"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 203), (169, 160)]),
            name: HashMap::from([(169, String::from("PZ.03"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 23,
            bcm: 11,
            cvm: String::from("SPI0_SCK"),
            tegra_soc: String::from("SPI1_SCK"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 206), (169, 163)]),
            name: HashMap::from([(169, String::from("PZ.06"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 24,
            bcm: 8,
            cvm: String::from("SPI0_CS0"),
            tegra_soc: String::from("SPI1_CS0_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 207), (169, 164)]),
            name: HashMap::from([(169, String::from("PZ.07"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 26,
            bcm: 7,
            cvm: String::from("SPI0_CS1"),
            tegra_soc: String::from("SPI1_CS1_N"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 133), (169, 105)]),
            name: HashMap::from([(169, String::from("PQ.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 29,
            bcm: 5,
            cvm: String::from("GPIO01"),
            tegra_soc: String::from("SOC_GPIO41"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 134), (169, 106)]),
            name: HashMap::from([(169, String::from("PQ.06"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 31,
            bcm: 6,
            cvm: String::from("GPIO11"),
            tegra_soc: String::from("SOC_GPIO42"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 136), (169, 108)]),
            name: HashMap::from([(169, String::from("PR.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 32,
            bcm: 12,
            cvm: String::from("GPIO07"),
            tegra_soc: String::from("SOC_GPIO44"),
            pwm_chip_sysfs: Some(String::from("32f0000.pwm")),
            pwm_id: Some(0),
        },
        PinDefinition {
            gpio: HashMap::from([(224, 105), (169, 84)]),
            name: HashMap::from([(169, String::from("PN.01"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 33,
            bcm: 13,
            cvm: String::from("GPIO13"),
            tegra_soc: String::from("SOC_GPIO54"),
            pwm_chip_sysfs: Some(String::from("3280000.pwm")),
            pwm_id: Some(0),
        },
        PinDefinition {
            gpio: HashMap::from([(224, 160), (169, 130)]),
            name: HashMap::from([(169, String::from("PU.00"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 35,
            bcm: 19,
            cvm: String::from("I2S0_FS"),
            tegra_soc: String::from("DAP5_FS"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 141), (169, 113)]),
            name: HashMap::from([(169, String::from("PR.05"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 36,
            bcm: 16,
            cvm: String::from("UART1_CTS"),
            tegra_soc: String::from("UART1_CTS"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 194), (169, 151)]),
            name: HashMap::from([(169, String::from("PY.02"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 37,
            bcm: 26,
            cvm: String::from("SPI1_MOSI"),
            tegra_soc: String::from("SPI3_MOSI"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 159), (169, 129)]),
            name: HashMap::from([(169, String::from("PT.07"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 38,
            bcm: 20,
            cvm: String::from("I2S0_DIN"),
            tegra_soc: String::from("DAP5_DIN"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
        PinDefinition {
            gpio: HashMap::from([(224, 158), (169, 128)]),
            name: HashMap::from([(169, String::from("PT.06"))]),
            chip_sysfs: String::from("2200000.gpio"),
            board: 40,
            bcm: 21,
            cvm: String::from("I2S0_DOUT"),
            tegra_soc: String::from("DAP5_DOUT"),
            pwm_chip_sysfs: None,
            pwm_id: None,
        },
    ];

    let jetson_xavier_pin_defs: [PinDefinition; 0] = [];

    let jetson_tx2_nx_pin_defs: [PinDefinition; 0] = [];

    let jetson_tx2_pin_defs: [PinDefinition; 0] = [];

    let jetson_tx1_pin_defs: [PinDefinition; 0] = [];

    let jetson_nano_pin_defs: [PinDefinition; 0] = [];

    if model == JETSON_ORIN {
        let pin_defs = jetson_orin_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == CLARA_AGX_XAVIER {
        let pin_defs = clara_agx_xavier_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_NX {
        let pin_defs = jetson_nx_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_XAVIER {
        let pin_defs = jetson_xavier_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_TX2_NX {
        let pin_defs = jetson_tx2_nx_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_TX2 {
        let pin_defs = jetson_tx2_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_TX1 {
        let pin_defs = jetson_tx1_pin_defs.to_vec();
        return Ok(pin_defs);
    } else if model == JETSON_NANO {
        let pin_defs = jetson_nano_pin_defs.to_vec();
        return Ok(pin_defs);
    }

    anyhow::bail!("No pin definitions found for model {}", model)
}

fn get_jetson_info(model: &str) -> Result<JetsonInfo, anyhow::Error> {
    if model == JETSON_ORIN {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("32768M, 65536M"),
            revision: String::from("Unknown"),
            ttype: String::from("JETSON_ORIN"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("A78AE"),
        };
        return Ok(jetson_info);
    } else if model == CLARA_AGX_XAVIER {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("16384M"),
            revision: String::from("Unknown"),
            ttype: String::from("CLARA_AGX_XAVIER"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM Carmel"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_NX {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("16384M, 8192M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson NX"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM Carmel"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_XAVIER {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("65536M, 32768M, 16384M, 8192M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson Xavier"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM Carmel"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_TX2_NX {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("4096M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson TX2 NX"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM A57 + Denver"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_TX2 {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("8192M, 4096M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson TX2"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM A57 + Denver"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_TX1 {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("4096M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson TX1"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM A57"),
        };
        return Ok(jetson_info);
    } else if model == JETSON_NANO {
        let jetson_info = JetsonInfo {
            p1_revision: 1,
            ram: String::from("4096M, 2048M"),
            revision: String::from("Unknown"),
            ttype: String::from("Jetson Nano"),
            manufacturer: String::from("NVIDIA"),
            processor: String::from("ARM A57"),
        };
        return Ok(jetson_info);
    }

    anyhow::bail!("No info found for model {}", model)
}

pub(crate) fn get_data() -> (
    String,
    JetsonInfo,
    HashMap<Mode, HashMap<u32, ChannelInfo>>,
) {
    let model = get_model().unwrap();

    let pin_defs: Vec<PinDefinition> = get_pin_defs(model.as_str()).unwrap();
    let jetson_info: JetsonInfo = get_jetson_info(model.as_str()).unwrap();

    let mut gpio_chip_dirs: HashMap<String, String> = HashMap::new();
    let mut gpio_chip_base: HashMap<String, u32> = HashMap::new();
    let mut gpio_chip_ngpio: HashMap<String, u32> = HashMap::new();
    let mut pwm_dirs: HashMap<String, String> = HashMap::new();

    let sysfs_prefixes = ["/sys/devices/", "/sys/devices/platform/"];

    // create an array of unique chip_sysfs values from the pin definitions
    let mut gpio_chip_names: Vec<String> = Vec::new();
    for pin_def in pin_defs.iter() {
        if !gpio_chip_names.contains(&pin_def.chip_sysfs) && !pin_def.chip_sysfs.is_empty() {
            gpio_chip_names.push(pin_def.chip_sysfs.clone());
        }
    }

    // find out the gpio sysdir, base, and ngpio values for each chip
    for gpio_chip_name in gpio_chip_names.iter() {
        let mut gpio_chip_dir: String = String::from("");
        for sysfs_prefix in sysfs_prefixes.iter() {
            let d = format!("{}{}", sysfs_prefix, gpio_chip_name);
            if Path::new(&d).exists() {
                gpio_chip_dir = d;
                break;
            }
        }

        if gpio_chip_dir == "" {
            // anyhow::bail!("Cannot find GPIO chip {}", gpio_chip_name);
        }

        gpio_chip_dirs.insert(gpio_chip_name.clone(), gpio_chip_dir.clone());
        let gpio_chip_gpio_dir = gpio_chip_dir + "/gpio";
        // for each file in the directory
        for entry in fs::read_dir(&gpio_chip_gpio_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            // check if the file name starts with "gpiochip"
            if !file_name.starts_with("gpiochip") {
                continue;
            }
            let base_fn = format!("{}/{}/base", gpio_chip_gpio_dir, file_name);
            let base = string_to_uint(read_file_to_string(&base_fn));
            gpio_chip_base.insert(gpio_chip_name.clone(), base);

            let ngpio_fn = format!("{}/{}/ngpio", gpio_chip_gpio_dir, file_name);
            let ngpio = string_to_uint(read_file_to_string(&ngpio_fn));
            gpio_chip_ngpio.insert(gpio_chip_name.clone(), ngpio);

            break;
        }
    }

    let mut pwm_chip_names: Vec<String> = Vec::new();
    for pin_def in pin_defs.iter() {
        if pin_def.pwm_chip_sysfs.is_some()
            && !pwm_chip_names.contains(&pin_def.pwm_chip_sysfs.as_ref().unwrap())
            && !pin_def.pwm_chip_sysfs.as_ref().unwrap().is_empty()
        {
            pwm_chip_names.push(pin_def.pwm_chip_sysfs.as_ref().unwrap().clone());
        }
    }

    for pwm_chip_name in pwm_chip_names.iter() {
        let mut pwm_chip_dir: String = String::from("");
        for sysfs_prefix in sysfs_prefixes.iter() {
            let d = format!("{}{}", sysfs_prefix, pwm_chip_name);
            if Path::new(&d).exists() {
                pwm_chip_dir = d;
                break;
            }
        }

        // Some PWM controllers aren't enabled in all versions of the DT. In
        // this case, just hide the PWM function on this pin, but let all other
        // aspects of the library continue to work.
        if pwm_chip_dir == "" {
            continue;
        }

        let pwm_chip_pwm_dir = pwm_chip_dir + "/pwm";
        if !Path::new(&pwm_chip_pwm_dir).exists() {
            continue;
        }

        // for each file in the directory
        for entry in fs::read_dir(&pwm_chip_pwm_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            // check if the file name starts with "gpiochip"
            if !file_name.starts_with("pwmchip") {
                continue;
            }

            let pwm_chip_pwm_pwmchipn_dir = format!("{}/{}", pwm_chip_pwm_dir, file_name);
            pwm_dirs.insert(pwm_chip_name.clone(), pwm_chip_pwm_pwmchipn_dir.clone());

            break;
        }
    }

    // create a hashmap of channel info, mapping each GPIO pin to a ChannelInfo struct
    let mut board_data: HashMap<u32, ChannelInfo> = HashMap::new();
    let mut bcm_data: HashMap<u32, ChannelInfo> = HashMap::new();
    for pin_def in pin_defs.iter() {
        let ngpio = gpio_chip_ngpio.get(&pin_def.chip_sysfs).unwrap();
        let chip_relative_id = pin_def.gpio.get(ngpio).unwrap();
        let gpio = gpio_chip_base.get(&pin_def.chip_sysfs).unwrap() + chip_relative_id;
        let default_gpio_name = format!("gpio{}", gpio);
        let gpio_name = pin_def.name.get(ngpio).unwrap_or(&default_gpio_name);

        let mut pwm_chip_dir: Option<String> = None;
        if pin_def.pwm_chip_sysfs.is_some() {
            let pwm_chip_sysfs = pin_def.pwm_chip_sysfs.as_ref().unwrap();
            pwm_chip_dir = pwm_dirs.get(pwm_chip_sysfs).cloned();
        }

        let channel_board = ChannelInfo {
            channel: pin_def.board.clone(),
            gpio_chip_dir: gpio_chip_dirs.get(&pin_def.chip_sysfs).unwrap().clone(),
            gpio: pin_def.gpio.clone(),
            global_gpio: gpio.clone(),
            global_gpio_name: gpio_name.clone(),
            pwm_chip_dir: pwm_chip_dir.clone(),
            pwm_id: pin_def.pwm_id.clone(),
        };

        let channel_bcm = ChannelInfo {
            channel: pin_def.bcm.clone(),
            gpio_chip_dir: gpio_chip_dirs.get(&pin_def.chip_sysfs).unwrap().clone(),
            gpio: pin_def.gpio.clone(),
            global_gpio: gpio.clone(),
            global_gpio_name: gpio_name.clone(),
            pwm_chip_dir: pwm_chip_dir.clone(),
            pwm_id: pin_def.pwm_id.clone(),
        };

        board_data.insert(channel_board.channel, channel_board);
        bcm_data.insert(channel_bcm.channel, channel_bcm);
    }

    let mut channel_data: HashMap<Mode, HashMap<u32, ChannelInfo>> = HashMap::new();
    channel_data.insert(Mode::BOARD, board_data);
    channel_data.insert(Mode::BCM, bcm_data);

    (model, jetson_info, channel_data)
}
