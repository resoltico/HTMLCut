use std::process;

fn main() {
    if let Err(error) = xtask::main_entry() {
        eprintln!("xtask: {error}");
        process::exit(1);
    }
}
