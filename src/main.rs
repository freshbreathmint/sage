use std::{thread, time::Duration};

fn main() {
    loop {
        println!("Hello, Sage!");

        thread::sleep(Duration::from_secs(1));
    }
}
