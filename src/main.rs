#[sage_hot_lib::hot_lib(dylib = "lib")]
mod hot_lib {
    hot_functions_from_file!("crates/lib/src/lib.rs");
}

fn main() {
    loop {
        hot_lib::test();
    }
}
