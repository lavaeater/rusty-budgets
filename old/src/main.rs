use std::env;

fn main() {
    unsafe { env::set_var("RUST_LOG", "debug"); }
    api::main(None);
}
