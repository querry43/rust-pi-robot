extern crate assigato_remote;
extern crate ws;

use ws::{connect, Handler, Sender, Handshake};

struct Client {
    out: Sender,
}

impl Handler for Client {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        println!("connected");
        let m: assigato_remote::Message = assigato_remote::Message::PWMChannelState(assigato_remote::robot::PWMChannelState { channel: 3, position: 0.73 } );
        //let m: assigato_remote::Message = assigato_remote::Message::LEDDisplayState(assigato_remote::robot::LEDDisplayState { channel: 0, state: [ false, true, false, true, false, false, false, false, true, true, true, true, false, false, false, true ] } );
        self.out.send(m.to_string())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        let robot = assigato_remote::robot::RobotState::from(msg.as_text()?);
        println!("on_message: {:?}", robot);
        Ok(())
    }
}

fn main() {
    connect("ws://127.0.0.1:3012", |out| Client { out: out } ).unwrap()
}
