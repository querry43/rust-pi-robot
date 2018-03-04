extern crate assigato_remote;
extern crate ws;

use std::sync::{Arc, Mutex};
use ws::{listen, Handler, Sender, Handshake, CloseCode};

struct Server {
    out: Sender,
    robot: Arc<Mutex<assigato_remote::robot::Robot>>,
}

impl Handler for Server {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        println!("client connected: {:?}", self.out.token());
        let r = self.robot.lock().unwrap();
        self.out.send(r.to_string())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let m = assigato_remote::Message::from(msg.as_text().unwrap());
        let mut r = self.robot.lock().unwrap();
        match m {
            assigato_remote::Message::PWMChannel(pwm) => r.pwm_channels[pwm.channel as usize].position = pwm.position.clone(),
            assigato_remote::Message::LEDDisplay(led) => r.led_displays[led.channel as usize].state = led.state.clone(),
        }
        r.update().unwrap();
        self.out.broadcast(r.to_string())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            _ => println!("The client disconnected: {:?} {}", self.out.token(), reason),
        }
    }
}

use std::thread;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let robot = Arc::new(Mutex::new(assigato_remote::robot::Robot { ..Default::default() }));
    spawn_robot_update_thread(robot.clone());
    listen("127.0.0.1:3012", |out| Server { out: out, robot: robot.clone() } ).unwrap()
} 

fn spawn_robot_update_thread(robot: Arc<Mutex<assigato_remote::robot::Robot>>) {
    thread::spawn(move || {
        loop {
            let r = robot.lock().unwrap();
            r.update().unwrap();
            sleep(Duration::from_secs(1));
        }
    });
}
