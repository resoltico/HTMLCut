use super::*;
use htmlcut_tempdir::TempDir;

mod loading;
mod output;
mod prepared;
mod runtime;

struct RequestFileFixture {
    tempdir: TempDir,
    input_path: PathBuf,
    input: String,
    selector_definition_path: PathBuf,
    slice_definition_path: PathBuf,
    request_file_output_path: PathBuf,
}

fn request_file_fixture() -> RequestFileFixture {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let selector_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Text),
    ));
    let selector_definition_path = write_definition_file(
        tempdir.path(),
        "selector-request.json",
        &selector_definition,
    );

    let slice_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::slice(
            htmlcut_core::SliceSpec::new(
                htmlcut_core::SliceBoundary::new("<article>").expect("slice boundary"),
                htmlcut_core::SliceBoundary::new("</article>").expect("slice boundary"),
            )
            .with_boundary_inclusion(true, true),
        )
        .with_selection(SelectionSpec::single())
        .with_value(ValueSpec::Text),
    ));
    let slice_definition_path =
        write_definition_file(tempdir.path(), "slice-request.json", &slice_definition);
    let request_file_output_path = tempdir.path().join("request-file-output.json");

    RequestFileFixture {
        tempdir,
        input_path,
        input,
        selector_definition_path,
        slice_definition_path,
        request_file_output_path,
    }
}
