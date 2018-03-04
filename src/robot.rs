use i2c_pca9685::PCA9685;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;
use serde_json;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;


#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Robot {
    pub pwm_channels: [PWMChannel; 16],
    pub led_displays: [LEDDisplay; 2],
}

impl Default for Robot {
    fn default() -> Robot {
        Robot {
            pwm_channels: [
                PWMChannel { channel:  0, ..Default::default() },
                PWMChannel { channel:  1, ..Default::default() },
                PWMChannel { channel:  2, ..Default::default() },
                PWMChannel { channel:  3, ..Default::default() },
                PWMChannel { channel:  4, ..Default::default() },
                PWMChannel { channel:  5, ..Default::default() },
                PWMChannel { channel:  6, ..Default::default() },
                PWMChannel { channel:  7, ..Default::default() },
                PWMChannel { channel:  8, ..Default::default() },
                PWMChannel { channel:  9, ..Default::default() },
                PWMChannel { channel: 10, ..Default::default() },
                PWMChannel { channel: 11, ..Default::default() },
                PWMChannel { channel: 12, ..Default::default() },
                PWMChannel { channel: 13, ..Default::default() },
                PWMChannel { channel: 14, ..Default::default() },
                PWMChannel { channel: 15, ..Default::default() },
            ],
            led_displays: [
                LEDDisplay { channel: 0, clock_pin: 20, data_pin: 21, ..Default::default() },
                LEDDisplay { channel: 1, clock_pin: 19, data_pin: 26, ..Default::default() },
            ],
        }
    }
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
    pub fn update(&self) -> Result<(), LinuxI2CError> {
        //self.update_pwm_channels().unwrap();
        self.update_led_displays().unwrap();
        Ok(())
    }

    fn update_pwm_channels(&self) -> Result<(), LinuxI2CError> {
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
        let device_info = DeviceInfo::new().unwrap();
        println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

        let mut gpio = Gpio::new().unwrap();

        for i in 0..self.led_displays.len()-1 {
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

        Ok(())
    }
}
