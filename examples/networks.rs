extern crate psutils;

use psutils::{ConnType, Connections};

fn main() {
    println!(
        "Connections: {:?}",
        Connections::retrieve(&ConnType::Tcp, None)
    );
}
