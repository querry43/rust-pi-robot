extern crate assigato_remote;
extern crate ws;

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::thread;
use std::time::Duration;
use ws::{listen, Handler, Sender, Handshake, CloseCode};

struct Server {
    out: Sender,
    robot: Arc<Mutex<assigato_remote::robot::Robot>>,
}

impl Handler for Server {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        println!("on_open: {:?}", self.out.token());
        let r = self.robot.lock().unwrap();
        self.out.send(r.state.to_string())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let m = assigato_remote::Message::from(msg.as_text()?);
		println!("on_message: {:?}", m);
        let mut r = self.robot.lock().unwrap();

        let res = match m {
            assigato_remote::Message::PWMChannelState(msg) => r.update_pwm_channel(msg),
            assigato_remote::Message::LEDDisplayState(msg) => r.update_led_display(msg),
            assigato_remote::Message::RobotSpeak(msg) => r.robot_speak(msg),
        };

        match res {
            Ok(()) => (),
            Err(err) => println!("Encountered error: {:?}", err),
        }

        self.out.broadcast(r.state.to_string())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            _ => println!("The client disconnected: {:?} {}", self.out.token(), reason),
        }
    }
}


fn main() {
    let robot = Arc::new(Mutex::new(assigato_remote::robot::Robot::new().unwrap()));
    println!("{:?}", robot);
	spawn_robot_update_thread(robot.clone());
	listen("127.0.0.1:3012", |out| Server { out: out, robot: robot.clone() } ).unwrap()
} 

fn spawn_robot_update_thread(robot: Arc<Mutex<assigato_remote::robot::Robot>>) {
    thread::spawn(move || {
        loop {
            sleep(Duration::from_secs(1));
            println!("refresh");
            let mut r = robot.lock().unwrap();
            match r.refresh() {
                Err(err) => println!("refresh error: {:?}", err),
                Ok(_) => (),
            }
        }
    });
}
