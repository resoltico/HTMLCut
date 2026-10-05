// SPDX-License-Identifier: MPL-2.0
//! Real private CLI modules shared by development-only fuzz entrypoints.
#[path = "../../../crates/htmlcut-cli/src/app.rs"]
pub(crate) mod app;
#[path = "../../../crates/htmlcut-cli/src/bundle.rs"]
pub(crate) mod bundle;
#[path = "../../../crates/htmlcut-cli/src/bundle_io.rs"]
pub(crate) mod bundle_io;
#[path = "../../../crates/htmlcut-cli/src/command.rs"]
pub(crate) mod command;
#[path = "../../../crates/htmlcut-cli/src/command_diagnostics.rs"]
pub(crate) mod command_diagnostics;
#[path = "../../../crates/htmlcut-cli/src/input.rs"]
pub(crate) mod input;
#[path = "../../../crates/htmlcut-cli/src/publication.rs"]
pub(crate) mod publication;
