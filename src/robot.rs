use config::{ConfigError, Config, File, Environment};
use i2c_pca9685::PCA9685;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;
use rppal;
use serde_json;
use std::error::Error;
use std::fmt;
use std::io;
use std::process::Command;


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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RobotSpeak {
    pub quip: String,
}

impl<'a> From<&'a str> for RobotSpeak {
    fn from(s: &str) -> Self {
        serde_json::from_str(&s).unwrap()
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

impl<'a> From<&'a str> for RobotState {
    fn from(s: &str) -> Self {
        serde_json::from_str(&s).unwrap()
    }
}

impl fmt::Display for RobotState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

pub struct Robot {
    config: RobotConfig,
    pub state: RobotState,
    pwm: Option<PCA9685<LinuxI2CDevice>>,
    gpio: Option<Gpio>,
}

impl fmt::Debug for Robot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Robot {{ config: {:?}, state: {:?} }}", self.config, self.state)
    }
}

#[derive(Debug)]
pub struct ChannelError {
    channel: u8,
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel out of range: {}", self.channel)
    }
}

impl Error for ChannelError {
    fn description(&self) -> &str {
        "Channel out of range"
    }
}

#[derive(Debug)]
pub enum RobotError {
    ConfigError(ConfigError),
    LinuxI2CError(LinuxI2CError),
    RpalSystemError(rppal::system::Error),
    RpalGpioError(rppal::gpio::Error),
    ChannelError(ChannelError),
    IOError(io::Error),
}

impl From<ConfigError> for RobotError {
    fn from(err: ConfigError) -> RobotError {
        RobotError::ConfigError(err)
    }
}

impl From<LinuxI2CError> for RobotError {
    fn from(err: LinuxI2CError) -> RobotError {
        RobotError::LinuxI2CError(err)
    }
}

impl From<rppal::system::Error> for RobotError {
    fn from(err: rppal::system::Error) -> RobotError {
        RobotError::RpalSystemError(err)
    }
}

impl From<rppal::gpio::Error> for RobotError {
    fn from(err: rppal::gpio::Error) -> RobotError {
        RobotError::RpalGpioError(err)
    }
}

impl From<ChannelError> for RobotError {
    fn from(err: ChannelError) -> RobotError {
        RobotError::ChannelError(err)
    }
}

impl From<io::Error> for RobotError {
    fn from(err: io::Error) -> RobotError {
        RobotError::IOError(err)
    }
}

impl Robot {
    pub fn new() -> Result<Self, RobotError> {
        let config = RobotConfig::new()?;

        let state = RobotState {
            pwm_channels: config.clone().pwm_channels.iter().map(|x| PWMChannelState::from(x)).collect(),
            led_displays: config.clone().led_displays.iter().map(|x| LEDDisplayState::from(x)).collect(),
        };

        let mut pwm_option: Option<PCA9685<LinuxI2CDevice>> = None;
        let mut gpio_option: Option<Gpio> = None;

        if config.enable {
            let device_info = DeviceInfo::new()?;
            println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

            let i2cdev = LinuxI2CDevice::new("/dev/i2c-1", 0x40)?;
            let mut pwm = PCA9685::new(i2cdev)?;
            pwm.set_pwm_freq(60.0)?;
            pwm.set_all_pwm(0, 0)?;

            pwm_option = Some(pwm);

            let mut gpio = Gpio::new()?;
            for i in 0..config.led_displays.len() {
                gpio.set_mode(config.led_displays[i].clock_pin, Mode::Output);
                gpio.set_mode(config.led_displays[i].data_pin, Mode::Output);
                gpio.write(config.led_displays[i].clock_pin, Level::Low);
                gpio.write(config.led_displays[i].data_pin, Level::Low);
            }

            gpio_option = Some(gpio);
        }

        Ok(
            Robot {
                config: config,
                state: state,
                pwm: pwm_option,
                gpio: gpio_option,
            }
        )
    }

    pub fn update_pwm_channel(&mut self, pwm_channel: PWMChannelState) -> Result<(), ChannelError> {
        if pwm_channel.channel as usize >= self.state.pwm_channels.len() {
            Err(ChannelError { channel: pwm_channel.channel })
        } else {
            self.state.pwm_channels[pwm_channel.channel as usize].position = pwm_channel.position;
            Ok(())
        }
    }

    pub fn update_led_display(&mut self, led_display: LEDDisplayState) -> Result<(), ChannelError> {
        if led_display.channel as usize >= self.state.led_displays.len() {
            Err(ChannelError { channel: led_display.channel })
        } else {
            self.state.led_displays[led_display.channel as usize].state = led_display.state.clone();
            Ok(())
        }
    }

    pub fn robot_speak(&mut self, robot_speak: RobotSpeak) -> Result<(), RobotError> {
        if self.config.enable {
            Command::new("espeak")
                    .arg("-s")
                    .arg("120")
                    .arg(robot_speak.quip)
                    .spawn()?;
        }
        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), RobotError> {
        self.refresh_led_displays()?;
        self.refresh_pwm_channels()?;
        Ok(())
    }

    fn refresh_led_displays(&mut self) -> Result<(), RobotError> {
        for i in 0..self.state.led_displays.len() {
            if self.config.debug {
                println!("Updating led display channel {} to {:?}", i, self.state.led_displays[i].state);
            }

            for b in self.state.led_displays[i].state.iter().rev() {
                match self.gpio {
                    None => (),
                    Some(ref mut gpio) => {
                            if *b {
                                gpio.write(self.config.led_displays[i].data_pin, Level::Low);
                            } else {
                                gpio.write(self.config.led_displays[i].data_pin, Level::High);
                            }

                            gpio.write(self.config.led_displays[i].clock_pin, Level::High);
                            gpio.write(self.config.led_displays[i].clock_pin, Level::Low);
                    }
                }
            }
        }

        Ok(())
    }

    fn refresh_pwm_channels(&mut self) -> Result<(), RobotError> {
        for i in 0..self.state.pwm_channels.len() {
            let mut position = self.state.pwm_channels[i].position;
            let range = self.config.pwm_channels[i].high - self.config.pwm_channels[i].low;
            let val: u16 = (position * range as f32) as u16 + self.config.pwm_channels[i].low;

            if self.config.debug {
                println!("Updating pwm channel {} to position {} val {}", i, self.state.pwm_channels[i].position, val);
            }

            match self.pwm {
                None => (),
                Some(ref mut pwm) => pwm.set_pwm(i as u8, 0, val)?
            }
        }

        Ok(())
    }
}
