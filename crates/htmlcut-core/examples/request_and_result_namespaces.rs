#[path = "support/request_and_result_namespaces.rs"]
mod support;

fn main() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    support::write_request_and_result_namespace_summary(&mut writer)
}
