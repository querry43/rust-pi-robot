extern crate assigato_remote;

use std::sync::{Arc, Mutex};

fn main() {
    let robot = Arc::new(Mutex::new(assigato_remote::robot::Robot::new().unwrap()));
    println!("{:?}", robot);
} 
