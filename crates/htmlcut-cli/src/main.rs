use std::io::{self, Write};

fn main() {
    let code = match htmlcut_cli::run(
        std::env::args(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(
                io::stderr().lock(),
                "htmlcut: failed to write CLI output: {error}"
            );
            htmlcut_cli::EXIT_CODE_OUTPUT
        }
    };
    std::process::exit(code);
}
