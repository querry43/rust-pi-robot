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
use std::time::SystemTime;


#[derive(Clone, Debug, Deserialize, PartialEq)]
struct PWMChannelConfig {
    channel: u8,
    name: String,
    invert: bool,
    low: u16,
    high: u16,
    initial_position: f32,
    idle_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PWMChannelState {
    pub channel: u8,
    pub position: Option<f32>,
}

impl<'a> From<&'a PWMChannelConfig> for PWMChannelState {
    fn from(config: &PWMChannelConfig) -> Self {
        PWMChannelState {
            channel: config.channel,
            position: Some(config.initial_position),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShiftRegisterConfig {
    channel: u8,
    initial_state: Vec<bool>,
    clock_pin: u8,
    data_pin: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ShiftRegisterState {
    pub channel: u8,
    pub state: Vec<bool>,
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
    pwm_channel_last_activity: Vec<SystemTime>,
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

        let pwm_channel_last_activity = state.clone().pwm_channels.iter().map(|_| SystemTime::now()).collect();

        let mut this = Robot {
            config: config,
            state: state,
            pwm_channel_last_activity: pwm_channel_last_activity,
            pwm: None,
            gpio: None,
        };

        this.init_hardware()?;

        Ok(this)
    }

    fn init_hardware(&mut self) -> Result<(), RobotError> {
        if self.config.enable {
            let device_info = DeviceInfo::new()?;
            println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

            let i2cdev = LinuxI2CDevice::new("/dev/i2c-1", 0x40)?;
            let mut pwm = PCA9685::new(i2cdev)?;
            pwm.set_pwm_freq(60.0)?;
            pwm.set_all_pwm(0, 0)?;

            self.pwm = Some(pwm);

            let mut gpio = Gpio::new()?;
            for i in 0..self.config.shift_registers.len() {
                gpio.set_mode(self.config.shift_registers[i].clock_pin, Mode::Output);
                gpio.set_mode(self.config.shift_registers[i].data_pin, Mode::Output);
                gpio.write(self.config.shift_registers[i].clock_pin, Level::Low);
                gpio.write(self.config.shift_registers[i].data_pin, Level::Low);
            }

            self.gpio = Some(gpio);
        }

        Ok(())
    }

    pub fn update_pwm_channel(&mut self, pwm_channel: PWMChannelState) -> Result<(), RobotError> {
        if pwm_channel.channel as usize >= self.state.pwm_channels.len() {
            Err(RobotError::ChannelError(ChannelError { channel: pwm_channel.channel }))
        } else {
            self.state.pwm_channels[pwm_channel.channel as usize].position = pwm_channel.position;
            self.pwm_channel_last_activity[pwm_channel.channel as usize] = SystemTime::now();
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
            let shift_register_state = self.state.shift_registers[i].clone();
            let shift_register_config = self.config.shift_registers[i].clone();
            if self.config.debug {
                println!("Updating led display channel {} to {:?}", i, shift_register_state.state);
            }
            self.bitbang(
                shift_register_state.state,
                shift_register_config.clock_pin,
                shift_register_config.data_pin,
            )?;
        }

        Ok(())
    }

    fn bitbang(&mut self, data: Vec<bool>, clock_pin: u8, data_pin: u8) -> Result<(), RobotError> {
        match self.gpio {
            None => (),
            Some(ref mut gpio) => {
                for bit in data.iter().rev() {
                    if *bit {
                        gpio.write(data_pin, Level::Low);
                    } else {
                        gpio.write(data_pin, Level::High);
                    }

                    gpio.write(clock_pin, Level::High);
                    gpio.write(clock_pin, Level::Low);
                }
            },
        }

        Ok(())
    }

    fn refresh_pwm_channels(&mut self) -> Result<(), RobotError> {
        for i in 0..self.state.pwm_channels.len() {
            let val = self.pwm_value_from_config_and_state(&self.config.pwm_channels[i], &self.state.pwm_channels[i]);

            let seconds_since_activity = match self.pwm_channel_last_activity[i].elapsed() {
                Err(_) => u64::max_value(),
                Ok(duration) => duration.as_secs(),
            };

            match self.config.pwm_channels[i].idle_after_seconds {
                None => (),
                Some(idle_after_seconds) => {
                    if !self.state.pwm_channels[i].position.is_none() && seconds_since_activity > idle_after_seconds {
                        if self.config.debug {
                            println!("Idling pwm channel {}", i);
                        }
                        self.state.pwm_channels[i].position = None;
                    }
                }
            }

            if self.config.debug {
                println!("Updating pwm channel {} to position {:?} val {}", i, self.state.pwm_channels[i].position, val);
            }

            match self.pwm {
                None => (),
                Some(ref mut pwm) => pwm.set_pwm(i as u8, 0, val)?
            }
        }

        Ok(())
    }

    fn pwm_value_from_config_and_state(&self, config: &PWMChannelConfig, state: &PWMChannelState) -> u16 {
        let val: u16 = match state.position {
            None => 0,
            Some(mut position) => {
                if config.invert {
                    position *= -1.0;
                    position += 1.0;
                }
                let range = config.high - config.low;
                (position * range as f32) as u16 + config.low
            },
        };
        val
    }
}

#[cfg(test)]
mod robot {

extern crate tempfile;

use std::io::Write;

static ROBOT_CONFIG: &str = r#"
---
debug: false
enable: false

pwm_channels:
  - channel: 0
    name: Channel 0
    invert: true
    low: 100
    high: 600
    initial_position: 0
    idle_after_seconds: 60
  - channel: 1
    name: Channel 1
    invert: false
    low: 200
    high: 500
    initial_position: 0.5
shift_registers:
  - channel: 0
    clock_pin: 10
    data_pin: 11
    initial_state: [ false, true, false, true ]
  - channel: 1
    clock_pin: 20
    data_pin: 21
    initial_state: [ false, false, false, false ]
"#;

use PWMChannelConfig;
use PWMChannelState;
use ShiftRegisterConfig;
use ShiftRegisterState;

#[test]
fn it_constructs_a_robot_config() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();

    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();

    let robot = ::Robot::new(tempfile.path().to_str().unwrap()).unwrap();
    let robot_config = robot.config;
    let robot_state = robot.state;

    assert_eq!(robot_config.pwm_channels.len(), 2);
    assert_eq!(robot_config.pwm_channels[0], PWMChannelConfig { channel: 0, name: "Channel 0".to_string(), invert: true, low: 100, high: 600, initial_position: 0.0, idle_after_seconds: Some(60) });
    assert_eq!(robot_config.pwm_channels[1], PWMChannelConfig { channel: 1, name: "Channel 1".to_string(), invert: false, low: 200, high: 500, initial_position: 0.5, idle_after_seconds: None });

    assert_eq!(robot_state.pwm_channels.len(), 2);
    assert_eq!(robot_state.pwm_channels[0], PWMChannelState { channel: 0, position: Some(0.0) });
    assert_eq!(robot_state.pwm_channels[1], PWMChannelState { channel: 1, position: Some(0.5) });

    assert_eq!(robot_config.shift_registers.len(), 2);
    assert_eq!(robot_config.shift_registers[0], ShiftRegisterConfig {
        channel: 0,
        initial_state: vec![ false, true, false, true ],
        clock_pin: 10,
        data_pin: 11,
    });
    assert_eq!(robot_config.shift_registers[1], ShiftRegisterConfig {
        channel: 1,
        initial_state: vec![ false, false, false, false ],
        clock_pin: 20,
        data_pin: 21,
    });

    assert_eq!(robot_state.shift_registers.len(), 2);
    assert_eq!(robot_state.shift_registers[0], ShiftRegisterState {
        channel: 0,
        state: vec![ false, true, false, true ],
    });
    assert_eq!(robot_state.shift_registers[1], ShiftRegisterState {
        channel: 1,
        state: vec![ false, false, false, false ],
    });
}

#[test]
fn it_updates_pwm_channel_state() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let mut robot = ::Robot::new(tempfile.path().to_str().unwrap()).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, Some(0.0));
    assert_eq!(robot_state.pwm_channels[1].position, Some(0.5));

    robot.update_pwm_channel(::PWMChannelState { channel: 1, position: Some(0.0) }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, Some(0.0));
    assert_eq!(robot_state.pwm_channels[1].position, Some(0.0));

    robot.update_pwm_channel(::PWMChannelState { channel: 1, position: Some(1.0) }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, Some(0.0));
    assert_eq!(robot_state.pwm_channels[1].position, Some(1.0));
}

#[test]
fn it_updates_shift_register_state() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let mut robot = ::Robot::new(tempfile.path().to_str().unwrap()).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state, vec![ false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state, vec![ false, false, false, false ]);

    robot.update_shift_register(::ShiftRegisterState { channel: 1, state: vec![ true, true, true, true ] }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state, vec![ false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state, vec![ true, true, true, true ]);

    robot.update_shift_register(::ShiftRegisterState { channel: 1, state: vec![ false, false, false, false ] }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state, vec![ false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state, vec![ false, false, false, false ]);
}

#[test]
fn it_calculates_pwm_channel_absolute_position() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let robot = ::Robot::new(tempfile.path().to_str().unwrap()).unwrap();

    let mut config = PWMChannelConfig {
        channel: 0,
        name: "Channel 0".to_string(),
        invert: false,
        low: 100,
        high: 600,
        initial_position: 0.0,
        idle_after_seconds: None,
    };

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(0.0),
            },
        ),
        100,
    );

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(0.5),
            },
        ),
        350,
    );

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(1.0),
            },
        ),
        600,
    );

    config.invert = true;

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(0.0),
            },
        ),
        600,
    );

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(0.5),
            },
        ),
        350,
    );

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: Some(1.0),
            },
        ),
        100,
    );

    assert_eq!(
        robot.pwm_value_from_config_and_state(
            &config,
            &PWMChannelState {
                channel: 0,
                position: None,
            },
        ),
        0,
    );
}

} // mod robot
