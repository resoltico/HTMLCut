// SPDX-License-Identifier: MPL-2.0
//! Complete values and bounded compact JSON encoding.
use crate::Error;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

/// The result shape follows the contract shape.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Value {
    /// An absent optional match.
    Null,
    /// A lexical value.
    String(String),
    /// All matches in document order.
    List(Vec<Value>),
    /// Fields in byte order of name.
    Record(BTreeMap<String, Value>),
}

impl Value {
    /// Encodes complete compact UTF-8 JSON, without a trailing LF.
    pub fn to_json(&self, maximum: usize) -> Result<Vec<u8>, Error> {
        let mut output = BoundedOutput {
            bytes: Vec::new(),
            maximum,
        };
        serde_json::to_writer(&mut output, self)
            .map_err(|_| Error::limit("output_bytes", maximum))?;
        Ok(output.bytes)
    }
}

pub(crate) struct OutputSize {
    used: usize,
    maximum: usize,
}
impl OutputSize {
    pub(crate) fn new(maximum: usize) -> Self {
        Self { used: 0, maximum }
    }
    pub(crate) fn syntax(&mut self, bytes: usize) -> Result<(), Error> {
        if bytes > self.maximum.saturating_sub(self.used) {
            return Err(Error::limit("output_bytes", self.maximum));
        }
        self.used += bytes;
        Ok(())
    }
    pub(crate) fn value(
        &mut self,
        value: &impl Serialize,
        clock: &crate::limits::Clock,
    ) -> Result<(), Error> {
        struct Count<'a> {
            size: &'a mut OutputSize,
            clock: &'a crate::limits::Clock,
            error: Option<Error>,
        }
        impl Write for Count<'_> {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if let Err(e) = self
                    .clock
                    .tick()
                    .and_then(|()| self.size.syntax(bytes.len()))
                {
                    self.error = Some(e);
                    return Err(io::Error::other("encoding refused"));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut counter = Count {
            size: self,
            clock,
            error: None,
        };
        if serde_json::to_writer(&mut counter, value).is_err() {
            return Err(counter
                .error
                .unwrap_or_else(|| Error::new(crate::Code::Internal, "Result encoding failed.")));
        }
        clock.check()
    }
}
struct BoundedOutput {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("output byte limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Code;
    #[test]
    fn encoding_counts_escaped_bytes_and_keeps_unicode_and_field_order() {
        let value = Value::Record(BTreeMap::from([
            ("z".into(), Value::Null),
            ("a".into(), Value::String("é\n\"".into())),
        ]));
        let expected = "{\"a\":\"é\\n\\\"\",\"z\":null}".as_bytes();
        assert_eq!(value.to_json(expected.len()).unwrap(), expected);
        let error = value.to_json(expected.len() - 1).unwrap_err();
        assert_eq!(error.error, Code::LimitExceeded);
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some((expected.len() - 1).into()));
    }
}
