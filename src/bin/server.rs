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
            assigato_remote::Message::PWMChannel(pwm) => r.pwm_channels[pwm.channel as usize] = pwm,
            assigato_remote::Message::LEDDisplay(led) => println!("got led update: {:?}", led),
        }
        self.out.broadcast(r.to_string())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            _ => println!("The client disconnected: {:?} {}", self.out.token(), reason),
        }
    }
}

fn main() {
    let robot = Arc::new(Mutex::new(assigato_remote::robot::Robot { ..Default::default() }));
    listen("127.0.0.1:3012", |out| Server { out: out, robot: robot.clone() } ).unwrap()
} 
