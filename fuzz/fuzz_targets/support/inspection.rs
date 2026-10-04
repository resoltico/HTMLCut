// SPDX-License-Identifier: MPL-2.0
use arbitrary::Arbitrary;
#[derive(Arbitrary, Debug)]
pub struct InspectionInput {
    html: String,
    samples: u8,
    selector: String,
}
pub fn drive(input: InspectionInput) {
    let Some(document) = crate::snapshot::document(&input.html) else {
        return;
    };
    let css = crate::snapshot::text(&input.selector, 1024);
    let first = document.inspect(css, u32::from(input.samples % 12));
    let second = document.inspect(css, u32::from(input.samples % 12));
    assert_eq!(first, second);
    if let Ok(result) = first {
        assert!(result.samples.len() <= 10);
        assert!(
            result
                .samples
                .iter()
                .all(|s| s.text.chars().count() <= 160 && s.attributes.len() <= 8)
        );
        assert_eq!(
            result.samples_complete,
            result.count as usize == result.samples.len()
        );
    }
}
