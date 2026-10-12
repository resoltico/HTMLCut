// SPDX-License-Identifier: MPL-2.0
#![deny(unsafe_code)]

mod app;
mod command;
mod command_diagnostics;
mod input;
#[allow(unsafe_code)]
mod native_resource;
mod publication;
mod stdio;
#[cfg(test)]
#[path = "tests/extraction_contract.rs"]
mod tests;
mod worker;

fn main() {
    if std::env::var_os(worker::MODE).is_some() {
        std::process::exit(worker::child_main());
    }
    std::process::exit(app::run(
        std::env::args_os(),
        &mut stdio::Input::default(),
        &mut stdio::Output::stdout(),
        &mut stdio::Output::stderr(),
    ));
}
