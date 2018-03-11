extern crate pi_robot;
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
    initial_state: [ false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true ]
  - channel: 1
    clock_pin: 20
    data_pin: 21
    initial_state: [ false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false ]
"#;

#[test]
fn it_constructs_a_robot_config() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();

    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();

    let robot = pi_robot::Robot::new(tempfile.path().to_str().unwrap()).unwrap();
    let robot_state = robot.state;

    assert_eq!(robot_state.pwm_channels.len(), 2);
    assert_eq!(robot_state.pwm_channels[0].channel, 0);
    assert_eq!(robot_state.pwm_channels[0].position, 0.0);
    assert_eq!(robot_state.pwm_channels[1].channel, 1);
    assert_eq!(robot_state.pwm_channels[1].position, 0.5);

    assert_eq!(robot_state.shift_registers.len(), 2);
    assert_eq!(robot_state.shift_registers[0].channel, 0);
    assert_eq!(robot_state.shift_registers[0].state,
        [ false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].channel, 1);
    assert_eq!(robot_state.shift_registers[1].state,
        [ false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false ]);
}

#[test]
fn it_updates_pwm_channel_state() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let mut robot = pi_robot::Robot::new(tempfile.path().to_str().unwrap()).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, 0.0);
    assert_eq!(robot_state.pwm_channels[1].position, 0.5);

    robot.update_pwm_channel(pi_robot::PWMChannelState { channel: 1, position: 0.0 }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, 0.0);
    assert_eq!(robot_state.pwm_channels[1].position, 0.0);

    robot.update_pwm_channel(pi_robot::PWMChannelState { channel: 1, position: 1.0 }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.pwm_channels[0].position, 0.0);
    assert_eq!(robot_state.pwm_channels[1].position, 1.0);
}

#[test]
fn it_updates_shift_register_state() {
    let mut tempfile = tempfile::NamedTempFileOptions::new().suffix(".yaml").create().unwrap();
    tempfile.write(ROBOT_CONFIG.as_bytes()).unwrap();
    let mut robot = pi_robot::Robot::new(tempfile.path().to_str().unwrap()).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state,
        [ false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state,
        [ false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false ]);

    robot.update_shift_register(pi_robot::ShiftRegisterState { channel: 1, state:
        [ true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true ]
    }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state,
        [ false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state,
        [ true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true ]);

    robot.update_shift_register(pi_robot::ShiftRegisterState { channel: 1, state:
        [ false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false ]
    }).unwrap();

    let robot_state = robot.state.clone();
    assert_eq!(robot_state.shift_registers[0].state,
        [ false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true ]);
    assert_eq!(robot_state.shift_registers[1].state,
        [ false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false ]);
}
