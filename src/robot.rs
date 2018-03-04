use config::{ConfigError, Config, File, Environment};
use i2c_pca9685::PCA9685;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;
use serde_json;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PWMChannel {
	pub channel: u8,
	pub position: f32,
}

impl Default for PWMChannel {
    fn default() -> PWMChannel {
        PWMChannel {
            channel: 0,
            position: 0.5,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LEDDisplay {
    pub channel: u8,
    pub state: [bool; 16],
    pub clock_pin: u8,
    pub data_pin: u8,
}

impl Default for LEDDisplay {
    fn default() -> LEDDisplay {
        LEDDisplay {
            channel: 0,
            state: [false; 16],
            clock_pin: 0,
            data_pin: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Robot {
    pub enable: bool,
    pub debug: bool,
    pub pwm_channels: Vec<PWMChannel>,
    pub led_displays: Vec<LEDDisplay>,
}

impl fmt::Display for Robot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl<'a> From<&'a str> for Robot {
    fn from(s: &str) -> Self {
        serde_json::from_str(&s).unwrap()
    }
}

impl Robot {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/robot.yaml"))?;
        s.merge(Environment::with_prefix("robot"))?;

        if s.get_bool("enable")? {
            let device_info = DeviceInfo::new().unwrap();
            println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());
        }

        s.try_into()
    }

    pub fn update(&self) -> Result<(), LinuxI2CError> {
        //self.update_pwm_channels().unwrap();
        self.update_led_displays().unwrap();
        Ok(())
    }

    fn update_pwm_channels(&self) -> Result<(), LinuxI2CError> {
        if ! self.enable { return Ok(()); }
        let i2cdevice = LinuxI2CDevice::new("/dev/i2c-1", 0x40)?;
        let mut pwm = PCA9685::new(i2cdevice)?;
        pwm.set_pwm_freq(60.0)?;
        pwm.set_all_pwm(0, 0)?;
    
        for x in 200..500 {
            pwm.set_pwm(1, 0, x)?;
            sleep(Duration::from_millis(10));
        }
    
        for x in (200..500).rev() {
            pwm.set_pwm(1, 0, x)?;
            sleep(Duration::from_millis(10));
        }
    
        Ok(())
    }

    fn update_led_displays(&self) -> Result<(), ()> {
        for i in 0..self.led_displays.len() {
            if self.debug {
                println!("Updating led display channel {} to {:?}", i, self.led_displays[i].state);
            }

            if self.enable {
                let mut gpio = Gpio::new().unwrap();
                gpio.set_mode(self.led_displays[i].clock_pin, Mode::Output);
                gpio.set_mode(self.led_displays[i].data_pin, Mode::Output);


                for b in self.led_displays[i].state.iter().rev() {
                    if *b {
                        gpio.write(self.led_displays[i].data_pin, Level::Low);
                    } else {
                        gpio.write(self.led_displays[i].data_pin, Level::High);
                    }

                    gpio.write(self.led_displays[i].clock_pin, Level::High);
                    gpio.write(self.led_displays[i].clock_pin, Level::Low);
                }
            }
        }

        Ok(())
    }
}
