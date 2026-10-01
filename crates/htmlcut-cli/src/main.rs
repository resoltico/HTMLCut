#![forbid(unsafe_code)]

mod app;
mod command;
mod command_diagnostics;
mod evidence;
mod input;
#[path = "input/http/media_type.rs"]
mod media_type;
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
