//! Bounded HTTP media parameters with quoted-string and quoted-pair interpretation.

use crate::input::{acquisition, limit};
use htmlcut_core::ExtractionError;

const MAX_HEADER_BYTES: usize = 64 * 1024;

fn invalid() -> ExtractionError {
    acquisition().with_cause(htmlcut_core::FailureCause::MediaType {})
}

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

struct Parameters<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Parameters<'_> {
    fn spaces(&mut self) {
        while self
            .bytes
            .get(self.offset)
            .is_some_and(|b| matches!(b, b' ' | b'\t'))
        {
            self.offset += 1;
        }
    }
    fn take_token(&mut self) -> Result<&[u8], ExtractionError> {
        let start = self.offset;
        while self.bytes.get(self.offset).is_some_and(|b| token(*b)) {
            self.offset += 1;
        }
        if start == self.offset {
            Err(invalid())
        } else {
            Ok(&self.bytes[start..self.offset])
        }
    }
    fn delimiter(&mut self, expected: u8) -> Result<(), ExtractionError> {
        if self.bytes.get(self.offset) != Some(&expected) {
            return Err(invalid());
        }
        self.offset += 1;
        Ok(())
    }
    fn value(&mut self) -> Result<Vec<u8>, ExtractionError> {
        if self.bytes.get(self.offset) != Some(&b'"') {
            return self.take_token().map(<[u8]>::to_vec);
        }
        self.offset += 1;
        let mut value = Vec::new();
        loop {
            let byte = *self.bytes.get(self.offset).ok_or_else(invalid)?;
            self.offset += 1;
            match byte {
                b'"' => return Ok(value),
                b'\\' => {
                    let next = *self.bytes.get(self.offset).ok_or_else(invalid)?;
                    self.offset += 1;
                    if !(matches!(next, b' ' | b'\t') || next >= 0x21) || next == 0x7f {
                        return Err(invalid());
                    }
                    value.push(next);
                }
                b'\t' | b' ' | b'!' | 0x23..=0x5b | 0x5d..=0xff if byte != 0x7f => value.push(byte),
                _ => return Err(invalid()),
            }
        }
    }
}

pub(super) fn charset(header: Option<&[u8]>) -> Result<Option<String>, ExtractionError> {
    let Some(bytes) = header else {
        return Ok(None);
    };
    if bytes.len() > MAX_HEADER_BYTES {
        return Err(limit("acquisition"));
    }
    let mut parser = Parameters { bytes, offset: 0 };
    parser.spaces();
    parser.take_token()?;
    parser.delimiter(b'/')?;
    parser.take_token()?;
    let mut charset: Option<String> = None;
    loop {
        parser.spaces();
        if parser.offset == bytes.len() {
            return Ok(charset);
        }
        parser.delimiter(b';')?;
        parser.spaces();
        if parser.offset == bytes.len() || parser.bytes.get(parser.offset) == Some(&b';') {
            continue;
        }
        let name = parser.take_token()?.to_vec();
        parser.delimiter(b'=')?;
        let value = parser.value()?;
        if name.eq_ignore_ascii_case(b"charset") {
            let value = std::str::from_utf8(&value)
                .map_err(|_| invalid())?
                .to_ascii_lowercase();
            if value.is_empty() {
                return Err(invalid());
            }
            if charset.as_ref().is_some_and(|old| old != &value) {
                return Err(invalid());
            }
            charset = Some(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_media_values_have_closed_safe_causes() {
        let cases: &[&[u8]] = &[
            b"text/html; note=\"bad\\\x00\"",
            b"text/html; note=\"bad\\\x7f\"",
            b"text/html; charset=\"\"",
            b"text/html; charset=\"\xff\"",
            b"text/html; note=\"trailing\\",
            b"text/html; note=\"bad\x7f\"",
        ];
        for header in cases {
            let error = charset(Some(header)).unwrap_err();
            assert_eq!(error.code.exit_class(), 5);
            assert_eq!(
                error.evidence.cause,
                Some(htmlcut_core::FailureCause::MediaType {})
            );
        }
        assert_eq!(charset(None).unwrap(), None);
        assert_eq!(charset(Some(b"text/html;; note=ok")).unwrap(), None);
        assert_eq!(
            charset(Some(b"text/html; note=\"\xff\"; charset=utf-8")).unwrap(),
            Some("utf-8".into())
        );
        assert_eq!(
            charset(Some(&vec![b'a'; MAX_HEADER_BYTES + 1]))
                .unwrap_err()
                .code
                .exit_class(),
            4
        );
    }

    #[test]
    fn quoted_parameters_do_not_become_charset_declarations() {
        for header in [
            r#"text/html; note="x;charset=windows-1252""#,
            r#"text/html; note="x;charset=unknown";"#,
        ] {
            assert_eq!(charset(Some(header.as_bytes())).unwrap(), None);
        }
        for header in [
            r#"text/html; charset=utf-8; note="x;charset=windows-1252""#,
            r#"text/html; note="x;charset=windows-1252"; charset="u\tf-8""#,
            "text/html; charset=UTF-8; charset=utf-8",
        ] {
            assert_eq!(
                charset(Some(header.as_bytes())).unwrap(),
                Some("utf-8".into())
            );
        }
        for header in [
            "text/html; charset=utf-8; charset=windows-1252",
            "text/html; note=\"unterminated",
            "text/html; charset=",
            "text/html; note=\"bad\r\nvalue\"",
            "text html",
        ] {
            assert!(charset(Some(header.as_bytes())).is_err(), "{header}");
        }
    }
}
