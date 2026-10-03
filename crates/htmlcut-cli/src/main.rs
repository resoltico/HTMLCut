#![forbid(unsafe_code)]

mod app;
mod bundle;
mod bundle_io;
mod command;
mod command_diagnostics;
mod input;
mod operation_metadata;
mod publication;
mod stdio;
#[cfg(test)]
#[path = "tests/extraction_contract.rs"]
mod tests;

fn main() {
    std::process::exit(app::run(
        std::env::args_os(),
        &mut stdio::Input::default(),
        &mut stdio::Output::stdout(),
        &mut stdio::Output::stderr(),
    ));
}
