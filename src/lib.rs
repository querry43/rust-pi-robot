extern crate config;
extern crate i2c_pca9685;
extern crate i2cdev;
extern crate rppal;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use config::{ConfigError, Config, File, Environment};
use i2c_pca9685::PCA9685;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;
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
pub struct ShiftRegisterConfig {
    channel: u8,
    initial_state: [bool; 16],
    clock_pin: u8,
    data_pin: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShiftRegisterState {
    pub channel: u8,
    pub state: [bool; 16],
}

impl<'a> From<&'a ShiftRegisterConfig> for ShiftRegisterState {
    fn from(config: &ShiftRegisterConfig) -> Self {
        ShiftRegisterState {
            channel: config.channel,
            state: config.initial_state.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RobotSpeak {
    pub quip: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RobotConfig {
    enable: bool,
    debug: bool,
    pwm_channels: Vec<PWMChannelConfig>,
    shift_registers: Vec<ShiftRegisterConfig>,
}

impl RobotConfig {
    pub fn new(config_file: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(config_file))?;
        s.merge(Environment::with_prefix("robot"))?;

        s.try_into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RobotState {
    pub pwm_channels: Vec<PWMChannelState>,
    pub shift_registers: Vec<ShiftRegisterState>,
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
    pub fn new(config_file: &str) -> Result<Self, RobotError> {
        let config = RobotConfig::new(config_file)?;

        let state = RobotState {
            pwm_channels: config.clone().pwm_channels.iter().map(|x| PWMChannelState::from(x)).collect(),
            shift_registers: config.clone().shift_registers.iter().map(|x| ShiftRegisterState::from(x)).collect(),
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
            for i in 0..config.shift_registers.len() {
                gpio.set_mode(config.shift_registers[i].clock_pin, Mode::Output);
                gpio.set_mode(config.shift_registers[i].data_pin, Mode::Output);
                gpio.write(config.shift_registers[i].clock_pin, Level::Low);
                gpio.write(config.shift_registers[i].data_pin, Level::Low);
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

    pub fn update_pwm_channel(&mut self, pwm_channel: PWMChannelState) -> Result<(), RobotError> {
        if pwm_channel.channel as usize >= self.state.pwm_channels.len() {
            Err(RobotError::ChannelError(ChannelError { channel: pwm_channel.channel }))
        } else {
            self.state.pwm_channels[pwm_channel.channel as usize].position = pwm_channel.position;
            Ok(())
        }
    }

    pub fn update_shift_register(&mut self, shift_register: ShiftRegisterState) -> Result<(), RobotError> {
        if shift_register.channel as usize >= self.state.shift_registers.len() {
            Err(RobotError::ChannelError(ChannelError { channel: shift_register.channel }))
        } else {
            self.state.shift_registers[shift_register.channel as usize].state = shift_register.state.clone();
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
        self.refresh_shift_registers()?;
        self.refresh_pwm_channels()?;
        Ok(())
    }

    fn refresh_shift_registers(&mut self) -> Result<(), RobotError> {
        for i in 0..self.state.shift_registers.len() {
            if self.config.debug {
                println!("Updating led display channel {} to {:?}", i, self.state.shift_registers[i].state);
            }

            for b in self.state.shift_registers[i].state.iter().rev() {
                match self.gpio {
                    None => (),
                    Some(ref mut gpio) => {
                            if *b {
                                gpio.write(self.config.shift_registers[i].data_pin, Level::Low);
                            } else {
                                gpio.write(self.config.shift_registers[i].data_pin, Level::High);
                            }

                            gpio.write(self.config.shift_registers[i].clock_pin, Level::High);
                            gpio.write(self.config.shift_registers[i].clock_pin, Level::Low);
                    }
                }
            }
        }

        Ok(())
    }

    fn refresh_pwm_channels(&mut self) -> Result<(), RobotError> {
        for i in 0..self.state.pwm_channels.len() {
            let mut position = self.state.pwm_channels[i].position;
            if self.config.pwm_channels[i].invert {
                position *= -1.0;
                position += 1.0;
            }
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
