# rust-pi-robot

[![Build status](https://img.shields.io/travis/querry43/rust-pi-robot.svg)](https://travis-ci.org/querry43/rust-pi-robot)
[![License](https://img.shields.io/github/license/querry43/rust-pi-robot.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/pi-robot.svg)](https://crates.io/crates/pi-robot)
[![Documentation](https://docs.rs/pi-robot/badge.svg)](https://docs.rs/pi-robot)

## Synopsys

```rust
extern crate pi_robot;
extern crate tempfile;

use std::io::Write;

// this config file would normally be written to disk but it is inline here as an example
static ROBOT_CONFIG: &str = "
---
debug: false

enable: false

pwm_channels:
  - channel: 0
    name: Left Arm
    invert: true
    low: 100
    high: 600
    initial_position: 0
    idle_after_seconds: 60
  - channel: 1
    name: Right Arm
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
";

fn main() {
    // create a temporary config file
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let config_file_path = tempfile.path().to_str().unwrap();

    // construct a robot
    let mut robot = pi_robot::Robot::new(config_file_path).unwrap();

    // make it dance!!
    robot.update_pwm_channel(pi_robot::PWMChannelState { channel: 1, position: Some(0.5) }).unwrap();
    robot.update_shift_register(pi_robot::ShiftRegisterState { channel: 1, state: vec![ true, true, true, true ] }).unwrap();
}
```

## Configuration

See the example config file inlined in the Synopsis.  Many of these values can be defaulted in the yaml config and later overwritten via env variables.

### Enable debug logging

```
debug: true
```

or

```
export ROBOT_DEBUG=1
```

### Enable hardware control

```
enable: true
```

or

```
export ROBOT_ENABLE=1
```

### PWM Channels using i2c-pca9685

PWM channels must be specified in sequential order including unused channels.  initial_position is between 0.0 and 1.0 denoting a position between low and high.

```
pwm_channels:
  - channel: 0
    name: Left Arm
    invert: true
    low: 100
    high: 600
    initial_position: 0
    idle_after_seconds: 60
  - channel: 1
    name: Right Arm
    invert: false
    low: 200
    high: 500
    initial_position: 0.5
```

### Shift register control

Shift registers must be specified in sequential order without skipping channels.  initial_state is the on/off value of each bit.

```
shift_registers:
  - channel: 0
    clock_pin: 10
    data_pin: 11
    initial_state: [ false, true, false, true ]
  - channel: 1
    clock_pin: 20
    data_pin: 21
    initial_state: [ false, false, false, false ]
```

## Troubleshooting

For warnings like:

```
ALSA lib pcm.c:2495:(snd_pcm_open_noupdate) Unknown PCM cards.pcm.modem
ALSA lib pcm.c:2495:(snd_pcm_open_noupdate) Unknown PCM cards.pcm.phoneline
```

edit /usr/share/alsa/alsa.conf and set channels to the default like

```
pcm.front cards.pcm.default
```
