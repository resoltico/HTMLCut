#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support/cli.rs"]
mod cli;

fuzz_target!(|input: &[u8]| {
    cli::drive(input);
});
