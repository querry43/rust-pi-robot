extern crate assigato_remote;
extern crate ws;

use ws::{connect, Handler, Sender, Handshake};

struct Client {
    out: Sender,
}

impl Handler for Client {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        //let m: assigato_remote::Message = { assigato_remote::Message::PWMChannel(assigato_remote::robot::PWMChannel { channel: 3, position: 0.73, ..Default::default() } ) };
        let m: assigato_remote::Message = { assigato_remote::Message::LEDDisplay(assigato_remote::robot::LEDDisplay { channel: 0, state: [ false, true, false, true, false, false, false, false, true, true, true, true, false, false, false, true ], ..Default::default() } ) };
        self.out.send(m.to_string())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let robot = assigato_remote::robot::Robot::from(msg.as_text().unwrap());
        println!("robot: {:?}", robot);
        Ok(())
    }
}

fn main() {
    connect("ws://127.0.0.1:3012", |out| Client { out: out } ).unwrap()
} 
