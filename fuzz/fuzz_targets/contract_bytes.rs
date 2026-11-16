// SPDX-License-Identifier: AGPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes)
        && let Err(error) = htmlcut_core::Contract::from_json(text, &htmlcut_core::Stop::none())
    {
        assert_ne!(error.code(), htmlcut_core::ErrorCode::Internal, "{error:?}");
    }
});
