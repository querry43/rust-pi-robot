extern crate config;
extern crate i2c_pca9685;
extern crate i2cdev;
extern crate rppal;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::fmt;

pub mod robot;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PWMChannel(robot::PWMChannel),
    LEDDisplay(robot::LEDDisplay),
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
