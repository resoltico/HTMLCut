/// Formats a byte size using friendly IEC binary units.
pub fn format_byte_size(bytes: usize) -> String {
    const KIBIBYTE: u128 = 1024;
    const MEBIBYTE: u128 = KIBIBYTE * KIBIBYTE;
    const GIBIBYTE: u128 = MEBIBYTE * KIBIBYTE;
    const UNITS: [(&str, u128); 3] = [("GiB", GIBIBYTE), ("MiB", MEBIBYTE), ("KiB", KIBIBYTE)];

    if bytes == 1 {
        return "1 byte".to_owned();
    }

    let bytes = bytes as u128;
    for (label, unit_size) in UNITS {
        if bytes < unit_size {
            continue;
        }

        let tenths = ((bytes * 10) + (unit_size / 2)) / unit_size;
        let whole = tenths / 10;
        let fractional = tenths % 10;
        return if fractional == 0 {
            format!("{whole} {label}")
        } else {
            format!("{whole}.{fractional} {label}")
        };
    }

    format!("{bytes} bytes")
}
