// SPDX-License-Identifier: MPL-2.0
//! Shared local application boundary for terminal and graphical consumers.
#![deny(unsafe_code)]
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("htmlcut-runtime supports macOS, Linux and Windows native hosts");
#[allow(unsafe_code)]
mod authorization;
mod job;
mod local_io;
use htmlcut_core::{Contract, Document, Source};
pub use htmlcut_core::{
    DEFAULT_READING, Error, ErrorCode, ErrorFacts, ExportFormat, Limits, OutlineOptions,
    OutlineReport, QueryCardinality, QuickQuery, Value,
};
pub use job::{CancelOutcome, Completion, Job, JobControl, Supervision};
use std::{path::PathBuf, sync::Arc};

/// Caller-owned bytes or a concrete local source.
#[derive(Clone, Debug)]
pub enum Input {
    Bytes(Vec<u8>),
    File(PathBuf),
    Stdin { allow_terminal: bool },
}
/// Contract acquisition, compiled before HTML acquisition.
#[derive(Clone, Debug)]
pub enum ContractInput {
    Json(String),
    File(PathBuf),
    Stdin,
    Quick(QuickQuery),
}
/// The closed application operation set.
#[derive(Clone, Debug)]
pub enum Operation {
    Extract {
        contract: ContractInput,
        format: ExportFormat,
    },
    Outline(OutlineOptions),
    Schema,
}
/// Where one complete result goes.
#[derive(Clone, Debug)]
pub enum Destination {
    Return,
    Stdout,
    File(PathBuf),
}
/// A complete application request. Policy defaults have one engine authority.
#[derive(Clone, Debug)]
pub struct Request {
    pub operation: Operation,
    pub input: Input,
    pub encoding: Option<String>,
    pub base_url: Option<String>,
    pub limits: Limits,
    pub destination: Destination,
}
impl Request {
    /// Refuses purely syntactic competing standard-input claims without host lookup.
    pub fn validate_syntax(&self) -> Result<(), Error> {
        let contract_stdin = match &self.operation {
            Operation::Extract {
                contract: ContractInput::Stdin,
                ..
            } => true,
            Operation::Extract {
                contract: ContractInput::File(path),
                ..
            } => local_io::obvious_stdin_alias(path),
            Operation::Extract {
                contract: ContractInput::Json(_) | ContractInput::Quick(_),
                ..
            }
            | Operation::Outline(_)
            | Operation::Schema => false,
        };
        if contract_stdin
            && match &self.input {
                Input::Stdin { .. } => true,
                Input::File(path) => local_io::obvious_stdin_alias(path),
                Input::Bytes(_) => false,
            }
        {
            return Err(Error::new(
                ErrorCode::InvalidRequest,
                "Contract stdin requires a distinct HTML source; competing standard input claims are refused before reading.",
            ));
        }
        Ok(())
    }
    /// Validates syntax before starting the request's one absolute job deadline.
    pub fn job(&self) -> Result<Job, Error> {
        self.validate_syntax()?;
        Job::new(self.limits.time_limit)
    }
}
/// Immutable document state, shareable across independent jobs.
#[derive(Clone, Debug)]
pub struct PreparedDocument(Arc<Document>);
/// A completed immutable semantic result, suitable for a later Save job.
#[derive(Clone, Debug)]
pub struct Computed {
    data: Arc<ResultData>,
}
#[derive(Clone, Debug)]
enum ResultData {
    Extraction(Value, ExportFormat),
    Outline(OutlineReport),
    Schema(Vec<u8>),
}
impl Computed {
    /// The complete value tree, when this is an extraction.
    pub fn value(&self) -> Option<&Value> {
        match self.data.as_ref() {
            ResultData::Extraction(value, _) => Some(value),
            ResultData::Outline(_) | ResultData::Schema(_) => None,
        }
    }
    /// Bounded grouped outline facts, when this is an outline operation.
    pub fn outline(&self) -> Option<&OutlineReport> {
        match self.data.as_ref() {
            ResultData::Outline(report) => Some(report),
            ResultData::Extraction(_, _) | ResultData::Schema(_) => None,
        }
    }
    /// Complete canonical UTF-8 schema JSON, including its framing LF.
    pub fn schema(&self) -> Option<&[u8]> {
        match self.data.as_ref() {
            ResultData::Schema(bytes) => Some(bytes),
            ResultData::Extraction(_, _) | ResultData::Outline(_) => None,
        }
    }
    fn encode(
        &self,
        maximum: usize,
        context: &htmlcut_core::ExecutionContext,
    ) -> Result<Vec<u8>, Error> {
        context.check()?;
        match self.data.as_ref() {
            ResultData::Extraction(value, format) => value.export(*format, maximum, context),
            ResultData::Outline(report) => report.render(maximum, context).map(String::into_bytes),
            ResultData::Schema(bytes) => admit_bytes(bytes, maximum),
        }
    }
}
fn admit_bytes(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, Error> {
    if bytes.len() > maximum {
        return Err(Error::limit("output_bytes", maximum));
    }
    Ok(bytes.to_vec())
}
/// Complete direct-stream or named-file delivery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delivery {
    Delivered,
    Disconnected,
}
/// A job either returns semantic state or completes one delivery.
#[derive(Clone, Debug)]
pub enum Outcome {
    Returned(Computed),
    Delivered(Delivery),
}
impl Job {
    /// Preflights, acquires, computes, encodes and optionally publishes exactly once.
    pub fn execute(self, request: Request) -> Completion<Outcome> {
        // Recheck syntax for hosts that created a Job separately from Request::job.
        if let Err(error) = request.validate_syntax() {
            return self.finish(Err(error));
        }
        self.perform(|| {
            let context = self.context();
            let contract_stdin = match &request.operation {
                Operation::Extract { contract: ContractInput::Stdin, .. } => true,
                Operation::Extract { contract: ContractInput::File(path), .. } => local_io::stdin_alias(path),
                Operation::Extract { contract: ContractInput::Json(_) | ContractInput::Quick(_), .. } | Operation::Outline(_) | Operation::Schema => false,
            };
            context.check()?;
            let html_stdin = match &request.input { Input::Stdin { .. } => true, Input::File(path) => local_io::stdin_alias(path), Input::Bytes(_) => false };
            if contract_stdin && html_stdin { return Err(Error::new(ErrorCode::InvalidRequest, "Contract standard input requires a distinct HTML source; the path aliases descriptor 0.")); }
            let operation = compile(&request.operation, &context)?;
            let document = if matches!(operation, Compiled::Schema) { None } else {
                Some(prepare(&request.input, &request.encoding, &request.base_url, &request.limits, &context)?)
            };
            let result = compute(document.as_ref(), operation, &request.limits, &context)?;
            self.deliver(result, &request.destination, &request.limits, &context)
        })
    }
    /// Prepares immutable semantic state without persisting this job's execution policy.
    pub fn prepare(
        self,
        input: Input,
        encoding: Option<String>,
        base_url: Option<String>,
        limits: Limits,
    ) -> Completion<PreparedDocument> {
        self.perform(|| {
            prepare(&input, &encoding, &base_url, &limits, &self.context())
                .map(|document| PreparedDocument(Arc::new(document)))
        })
    }
    /// Executes against a shared document with fresh limits and cancellation.
    pub fn execute_prepared(
        self,
        document: &PreparedDocument,
        operation: Operation,
        limits: Limits,
        destination: Destination,
    ) -> Completion<Outcome> {
        self.perform(|| {
            let context = self.context();
            let operation = compile(&operation, &context)?;
            let result = compute(Some(&document.0), operation, &limits, &context)?;
            self.deliver(result, &destination, &limits, &context)
        })
    }
    /// Saves a previous result, re-admitting its canonical export under new policy.
    pub fn save(
        self,
        result: &Computed,
        destination: Destination,
        limits: Limits,
    ) -> Completion<Outcome> {
        self.perform(|| {
            let context = self.context();
            self.deliver(result.clone(), &destination, &limits, &context)
        })
    }
    /// Delivers host presentation bytes with the same destination protections.
    pub fn present(self, bytes: &[u8], destination: Destination) -> Completion<Delivery> {
        self.perform(|| self.publish(bytes, &destination))
    }
    fn deliver(
        &self,
        result: Computed,
        destination: &Destination,
        limits: &Limits,
        context: &htmlcut_core::ExecutionContext,
    ) -> Result<Outcome, Error> {
        let bytes = result.encode(limits.max_output_bytes, context)?;
        match destination {
            Destination::Return => Ok(Outcome::Returned(result)),
            Destination::Stdout | Destination::File(_) => {
                self.publish(&bytes, destination).map(Outcome::Delivered)
            }
        }
    }
    fn publish(&self, bytes: &[u8], destination: &Destination) -> Result<Delivery, Error> {
        self.check()?;
        match destination {
            Destination::Stdout => local_io::publish_timed(bytes, None, Some(self)),
            Destination::File(path) => local_io::publish_timed(bytes, Some(path), Some(self)),
            Destination::Return => Err(Error::new(
                ErrorCode::InvalidRequest,
                "Presentation bytes require a stream or file destination.",
            )),
        }
    }
}
enum Compiled {
    Extraction(Box<Contract>, ExportFormat),
    Outline(OutlineOptions),
    Schema,
}
fn compile(
    operation: &Operation,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Compiled, Error> {
    context.check()?;
    Ok(match operation {
        Operation::Extract { contract, format } => {
            let contract = match contract {
                ContractInput::Json(text) => match Contract::from_json(text, context) {
                    Ok(contract) => contract,
                    Err(mut error) => {
                        if error.error == ErrorCode::InvalidContract
                            && text.len() <= Contract::MAX_BYTES
                            && serde_json::from_str::<serde_json::Value>(text).is_err()
                        {
                            context.check()?;
                            if std::path::Path::new(text).is_file() {
                                error.file = Some(text.clone());
                            }
                        }
                        return Err(error);
                    }
                },
                ContractInput::Quick(query) => Contract::quick(query, context)?,
                ContractInput::File(path) => {
                    let bytes = local_io::read_file(path, Contract::MAX_BYTES, true, context)?;
                    let text = std::str::from_utf8(&bytes).map_err(|e| {
                        Error::new(
                            ErrorCode::InvalidContract,
                            format!("The contract is not UTF-8: {e}."),
                        )
                    })?;
                    Contract::from_json(text, context)?
                }
                ContractInput::Stdin => {
                    let bytes = local_io::read(
                        &mut std::io::stdin().lock(),
                        Contract::MAX_BYTES,
                        std::path::Path::new("-"),
                        true,
                        context,
                    )?;
                    let text = std::str::from_utf8(&bytes).map_err(|e| {
                        Error::new(
                            ErrorCode::InvalidContract,
                            format!("The contract is not UTF-8: {e}."),
                        )
                    })?;
                    Contract::from_json(text, context)?
                }
            };
            contract.admit_export(*format)?;
            Compiled::Extraction(Box::new(contract), *format)
        }
        Operation::Outline(options) => {
            options.validate(context)?;
            Compiled::Outline(options.clone())
        }
        Operation::Schema => Compiled::Schema,
    })
}
fn prepare(
    input: &Input,
    encoding: &Option<String>,
    base_url: &Option<String>,
    limits: &Limits,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Document, Error> {
    context.check()?;
    let acquired;
    let bytes = match input {
        Input::Bytes(bytes) => bytes.as_slice(),
        Input::File(path) => {
            acquired = local_io::read_file(path, limits.max_input_bytes, false, context)?;
            &acquired
        }
        Input::Stdin { allow_terminal } => {
            acquired = local_io::input(*allow_terminal, limits.max_input_bytes, context)?;
            &acquired
        }
    };
    context.check()?;
    let mut source = Source::new(bytes);
    source.encoding = encoding.as_deref();
    source.base_url = base_url.as_deref();
    Document::parse(source, &(limits).preparation(), context)
}
fn compute(
    document: Option<&Document>,
    operation: Compiled,
    limits: &Limits,
    context: &htmlcut_core::ExecutionContext,
) -> Result<Computed, Error> {
    let data = match operation {
        Compiled::Extraction(contract, format) => ResultData::Extraction(
            document
                .unwrap()
                .extract(&contract, &(limits).execution(), context)?,
            format,
        ),
        Compiled::Outline(options) => ResultData::Outline(document.unwrap().outline(
            &options,
            &(limits).execution(),
            context,
        )?),
        Compiled::Schema => {
            ResultData::Schema(Contract::export_schema(limits.max_output_bytes, context)?)
        }
    };
    Ok(Computed {
        data: Arc::new(data),
    })
}
