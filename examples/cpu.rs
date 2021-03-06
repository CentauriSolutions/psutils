extern crate psutils;

use psutils::system::cpu;

fn main() {
    println!("CPU Time:");

    println!("{:?}", cpu::times().unwrap());

    println!("CPU Specific information:");

    for time in cpu::cpu_time() {
        println!("\t{:?}", time);
    }
}
