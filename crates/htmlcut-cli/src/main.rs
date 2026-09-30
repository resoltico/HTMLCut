#![forbid(unsafe_code)]

mod app;
mod command;
mod evidence;
mod input;
mod operation_metadata;
mod publication;
#[cfg(test)]
#[path = "tests/extraction_contract.rs"]
mod tests;

fn main() {
    std::process::exit(app::run(
        std::env::args_os(),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    ));
}
