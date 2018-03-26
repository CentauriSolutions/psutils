extern crate psutils;

use psutils::{ConnType, Connections};

fn main() {
    Connections::retrieve(&ConnType::Tcp, None);
}
