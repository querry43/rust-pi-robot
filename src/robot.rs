use config::{ConfigError, Config, File, Environment};


#[derive(Clone, Debug, Deserialize)]
struct PWMChannelConfig {
    channel: u8,
    invert: bool,
    low: u16,
    high: u16,
    initial_position: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PWMChannelState {
    pub channel: u8,
    pub position: f32,
}

impl<'a> From<&'a PWMChannelConfig> for PWMChannelState {
    fn from(config: &PWMChannelConfig) -> Self {
        PWMChannelState {
            channel: config.channel,
            position: config.initial_position,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct LEDDisplayConfig {
    channel: u8,
    initial_state: [bool; 16],
    clock_pin: u8,
    data_pin: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LEDDisplayState {
    pub channel: u8,
    pub state: [bool; 16],
}

impl<'a> From<&'a LEDDisplayConfig> for LEDDisplayState {
    fn from(config: &LEDDisplayConfig) -> Self {
        LEDDisplayState {
            channel: config.channel,
            state: config.initial_state.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RobotConfig {
    enable: bool,
    debug: bool,
    pwm_channels: Vec<PWMChannelConfig>,
    led_displays: Vec<LEDDisplayConfig>,
}

impl RobotConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/robot.yaml"))?;
        s.merge(Environment::with_prefix("robot"))?;

        s.try_into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RobotState {
    pub pwm_channels: Vec<PWMChannelState>,
    pub led_displays: Vec<LEDDisplayState>,
}

#[derive(Clone, Debug)]
pub struct Robot {
    config: RobotConfig,
    state: RobotState,
}

impl Robot {
    pub fn new() -> Result<Self, ConfigError> {
        let config = RobotConfig::new()?;

        let state = RobotState {
            pwm_channels: config.clone().pwm_channels.iter().map(|x| PWMChannelState::from(x)).collect(),
            led_displays: config.clone().led_displays.iter().map(|x| LEDDisplayState::from(x)).collect(),
        };

        Ok(
            Robot {
                config: config,
                state: state,
            }
        )
    }

    pub fn update_pwm_channel(&self, pwm_channel: PWMChannelState) {
    }

    pub fn update_led_display(&self, led_display: LEDDisplayState) {
    }
}
