fn main() {
    test_fn();
}

#[sage_hot_lib::test_macro]
fn test_fn() {
    println!("Hello, Sage!");
}
