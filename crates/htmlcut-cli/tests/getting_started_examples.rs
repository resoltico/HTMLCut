mod support;
use support::invoke;

#[test]
fn basic_extraction_needs_only_index_and_one_description() {
    for args in [vec!["describe"], vec!["describe", "extract"]] {
        let output = invoke(&args, b"");
        assert!(output.status.success());
        assert!(output.stdout.len() < 4096);
    }
    let output = invoke(
        &["extract", "--stdin", "--css", "#amount", "--raw"],
        b"<p id='amount'>EUR 180</p>",
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"EUR 180");
}
