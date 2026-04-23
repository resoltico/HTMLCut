//! External Markdown doctest harnesses for the maintained public Rust examples.

#[doc = include_str!("../../../docs/architecture.md")]
#[allow(dead_code)]
pub struct ArchitectureGuideDoctests;

#[doc = include_str!("../../../docs/core.md")]
#[allow(dead_code)]
pub struct CoreGuideDoctests;

#[doc = include_str!("../../../docs/interop-v1.md")]
#[allow(dead_code)]
pub struct InteropV1GuideDoctests;

#[doc = include_str!("../../../docs/schema.md")]
#[allow(dead_code)]
pub struct SchemaGuideDoctests;

#[cfg(test)]
fn maintained_markdown_doctest_paths() -> [&'static str; 4] {
    [
        "../../../docs/architecture.md",
        "../../../docs/core.md",
        "../../../docs/interop-v1.md",
        "../../../docs/schema.md",
    ]
}

#[cfg(test)]
mod tests {
    use super::maintained_markdown_doctest_paths;

    #[test]
    fn maintained_markdown_doctest_inventory_stays_complete() {
        assert_eq!(
            maintained_markdown_doctest_paths(),
            [
                "../../../docs/architecture.md",
                "../../../docs/core.md",
                "../../../docs/interop-v1.md",
                "../../../docs/schema.md",
            ]
        );
    }
}
