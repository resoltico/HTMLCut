// SPDX-License-Identifier: MPL-2.0
//! Test fixture setup, using the public engine API.
#![allow(dead_code)]
use htmlcut_core::{Document, Error, ParseOptions, ResultLimits, Stop};
#[derive(Clone, Debug)]
pub struct TestOptions {
    pub max_input_bytes: usize,
    pub max_nodes: usize,
    pub max_reading_bytes: usize,
    pub max_output_bytes: usize,
    pub time_limit: Option<std::time::Duration>,
}
impl Default for TestOptions {
    fn default() -> Self {
        let p = ParseOptions::default();
        let r = ResultLimits::default();
        Self {
            max_input_bytes: p.max_input_bytes,
            max_nodes: p.max_nodes,
            max_reading_bytes: r.max_reading_bytes,
            max_output_bytes: r.max_output_bytes,
            time_limit: None,
        }
    }
}
impl TestOptions {
    pub fn preparation(&self) -> ParseOptions {
        ParseOptions {
            max_input_bytes: self.max_input_bytes,
            max_nodes: self.max_nodes,
            ..Default::default()
        }
    }
    pub fn execution(&self) -> ResultLimits {
        ResultLimits {
            max_reading_bytes: self.max_reading_bytes,
            max_output_bytes: self.max_output_bytes,
        }
    }
}
pub struct DocumentFixture<'a> {
    pub bytes: &'a [u8],
    pub encoding: Option<&'a str>,
    pub base_url: Option<&'a str>,
}
impl<'a> DocumentFixture<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            encoding: None,
            base_url: None,
        }
    }
}
pub fn parse_fixture(
    input: DocumentFixture<'_>,
    options: &ParseOptions,
    stop: &Stop,
) -> Result<Document, Error> {
    Document::parse(
        input.bytes,
        &ParseOptions {
            encoding: input.encoding.map(Into::into),
            base_url: input.base_url.map(Into::into),
            ..options.clone()
        },
        stop,
    )
}
pub fn test_stop(limit: Option<std::time::Duration>) -> Result<Stop, Error> {
    Ok(limit
        .map(Stop::after)
        .transpose()?
        .unwrap_or_else(Stop::none))
}

#[derive(Clone, Debug)]
pub struct QueryFixture {
    pub select: String,
    pub read: String,
    pub cardinality: htmlcut_core::Match,
}
pub fn compile_query(query: &QueryFixture, stop: &Stop) -> Result<htmlcut_core::Contract, Error> {
    stop.check()?;
    let read = query
        .read
        .parse()
        .map_err(|reason| Error::InvalidContract {
            path: Some("$.read".into()),
            reason,
        })?;
    htmlcut_core::Contract::compile(
        &htmlcut_core::NodeSpec::Node(Box::new(htmlcut_core::NodeObject {
            select: Some(query.select.clone()),
            read: Some(read),
            r#match: Some(query.cardinality),
            ..Default::default()
        })),
        stop,
    )
}
