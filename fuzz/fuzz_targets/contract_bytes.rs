// SPDX-License-Identifier: MPL-2.0
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes)
        && let Err(error) = htmlcut_core::Contract::from_json(text, &htmlcut_core::Stop::none())
    {
        assert_ne!(error.code(), htmlcut_core::ErrorCode::Internal, "{error:?}");
    }
});
