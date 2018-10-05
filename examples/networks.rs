extern crate psutils;

use psutils::{ConnType, Connections};

fn main() {
    println!("Connections:");
    for connection in Connections::retrieve(&ConnType::Tcp, None) {
        println!("{:?}", connection);
    }
}
