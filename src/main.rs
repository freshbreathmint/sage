#[sage_hot_lib::hot_lib(dylib = "test")]
mod hot_lib {
    hot_functions_from_file!("crates/test/src/lib.rs");
}

fn main() {
    hot_lib::test();
}
