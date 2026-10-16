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

/// Complete extraction representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    /// Compact JSON followed by LF.
    Json,
    /// Scalar/list string framing; absent optional values emit no bytes.
    Raw,
}
impl Value {
    /// Admits the canonical complete result, then renders the requested format.
    pub fn export(
        &self,
        format: ExportFormat,
        maximum: usize,
        context: &crate::ExecutionContext,
    ) -> Result<Vec<u8>, Error> {
        context.check()?;
        let clock = (context).clone();
        let mut canonical = OutputSize::new(maximum);
        canonical.syntax(1)?;
        canonical.value(self, &clock)?;
        if format == ExportFormat::Json {
            return encode_json(self, maximum, context);
        }
        let mut output = BoundedOutput {
            bytes: Vec::new(),
            maximum,
            clock: &clock,
            error: None,
        };
        let mut add = |value: &str| -> Result<(), Error> {
            output
                .write_all(value.as_bytes())
                .and_then(|()| output.write_all(b"\n"))
                .map_err(|_| {
                    output
                        .error
                        .take()
                        .unwrap_or_else(|| Error::limit("output_bytes", maximum))
                })
        };
        match self {
            Value::Null => (),
            Value::String(value) => add(value)?,
            Value::List(values) => {
                for (index, value) in values.iter().enumerate() {
                    match value {
                        Value::String(value) if !value.contains('\n') => add(value)?,
                        Value::String(_) => {
                            let mut error = Error::new(
                                crate::ErrorCode::UnrepresentableOutput,
                                "Raw lists require one LF-free string per record. Use canonical JSON output.",
                            );
                            error.path = Some("$".into());
                            error.row = Some(vec![index + 1]);
                            return Err(error);
                        }
                        Value::Null | Value::List(_) | Value::Record(_) => return Err(raw_shape()),
                    }
                }
            }
            Value::Record(_) => return Err(raw_shape()),
        }
        context.check()?;
        Ok(output.bytes)
    }
}
fn raw_shape() -> Error {
    Error::new(
        crate::ErrorCode::UnrepresentableOutput,
        "Raw export requires a value root containing strings or an absent optional value.",
    )
}
pub(crate) fn encode_json(
    value: &impl Serialize,
    maximum: usize,
    context: &crate::ExecutionContext,
) -> Result<Vec<u8>, Error> {
    context.check()?;
    let clock = (context).clone();
    let mut output = BoundedOutput {
        bytes: Vec::new(),
        maximum,
        clock: &clock,
        error: None,
    };
    serde_json::to_writer(&mut output, value)
        .and_then(|()| output.write_all(b"\n").map_err(serde_json::Error::io))
        .map_err(|_| {
            output
                .error
                .take()
                .unwrap_or_else(|| Error::limit("output_bytes", maximum))
        })?;
    context.check()?;
    Ok(output.bytes)
}

pub(crate) struct OutputSize {
    used: usize,
    maximum: usize,
}
impl OutputSize {
    pub(crate) fn new(maximum: usize) -> Self {
        Self { used: 0, maximum }
    }
    pub(crate) fn remaining(&self) -> usize {
        self.maximum.saturating_sub(self.used)
    }
    pub(crate) fn maximum(&self) -> usize {
        self.maximum
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
        clock: &crate::limits::ExecutionContext,
    ) -> Result<(), Error> {
        struct Count<'a> {
            size: &'a mut OutputSize,
            clock: &'a crate::limits::ExecutionContext,
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
            return Err(counter.error.unwrap_or_else(|| {
                Error::new(crate::ErrorCode::Internal, "Result encoding failed.")
            }));
        }
        clock.check()
    }
}
struct BoundedOutput<'a> {
    clock: &'a crate::limits::ExecutionContext,
    error: Option<Error>,
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for BoundedOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Err(error) = self.clock.tick() {
            self.error = Some(error);
            return Err(io::Error::other("encoding stopped"));
        }
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
    use crate::ErrorCode;
    #[test]
    fn encoding_counts_escaped_bytes_and_keeps_unicode_and_field_order() {
        let value = Value::Record(BTreeMap::from([
            ("z".into(), Value::Null),
            ("a".into(), Value::String("é\n\"".into())),
        ]));
        let expected = "{\"a\":\"é\\n\\\"\",\"z\":null}\n".as_bytes();
        assert_eq!(
            value
                .export(
                    crate::ExportFormat::Json,
                    expected.len(),
                    &crate::ExecutionContext::new(None).unwrap()
                )
                .unwrap(),
            expected
        );
        let error = value
            .export(
                crate::ExportFormat::Json,
                expected.len() - 1,
                &crate::ExecutionContext::new(None).unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.error, ErrorCode::LimitExceeded);
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some((expected.len() - 1).into()));
    }
}
