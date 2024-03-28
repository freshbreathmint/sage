#[cfg(feature = "reload")]
use hot_lib::*;
#[cfg(not(feature = "reload"))]
use lib::*;

#[cfg(feature = "reload")]
#[sage_hot_lib::hot_lib(dylib = "lib")]
mod hot_lib {
    hot_functions_from_file!("crates/lib/src/lib.rs");
}

fn main() {
    // Debug
    #[cfg(feature = "reload")]
    println!("Hot!");
    #[cfg(not(feature = "reload"))]
    println!("Static!");

    // Loop
    loop {
        test();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
