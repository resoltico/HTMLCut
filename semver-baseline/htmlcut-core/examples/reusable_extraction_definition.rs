#[path = "support/reusable_extraction_definition.rs"]
mod support;

fn main() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    support::write_reusable_extraction_definition(&mut writer)
}
