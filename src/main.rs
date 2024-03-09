use std::{thread, time::Duration};

fn main() {
    loop {
        do_needful()
    }
}

fn do_needful() {
    println!("Hello, Sage!");

    thread::sleep(Duration::from_secs(1));
}
