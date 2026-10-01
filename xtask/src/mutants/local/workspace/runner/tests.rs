use std::time::Duration;

#[test]
#[ignore = "only executed as an owned child by the healthy supervision test"]
fn child_waits_for_headroom_marker() {
    let marker = std::path::PathBuf::from(
        std::env::var_os("HTMLCUT_SUPERVISOR_MARKER").expect("marker path"),
    );
    let started = std::time::Instant::now();
    while !marker.is_file() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "headroom callback was not invoked"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
