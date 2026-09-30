use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
pub struct DiscoveryInput {
    html: String,
    page_size: u8,
    malformed_cursor: String,
}

pub fn drive(input: DiscoveryInput) {
    let Some(document) = crate::snapshot::document(&input.html) else {
        return;
    };
    let size = u32::from(input.page_size % 20 + 1);
    if let Ok(page) = document.inspect(size, None) {
        if let Some(cursor) = page.next_cursor {
            let _ = document.inspect(size, Some(&cursor));
        }
        for element in page.elements.iter().take(2) {
            let _ = document.propose(&element.handle, size);
        }
    }
    let _ = document.inspect(
        size,
        Some(crate::snapshot::text(&input.malformed_cursor, 1024)),
    );
}
