extern crate config;
extern crate i2c_pca9685;
extern crate i2cdev;
extern crate rppal;
extern crate serde_json;

use std::fmt;

#[macro_use]
extern crate serde_derive;

pub mod robot;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PWMChannelState(robot::PWMChannelState),
    LEDDisplayState(robot::LEDDisplayState),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl<'a> From<&'a str> for Message {
    fn from(s: &str) -> Self {
        serde_json::from_str(&s).unwrap()
    }
}

