use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use htmlcut_tempdir::{TempDir, tempdir};

use crate::model::DynResult;

use super::runtime::{documented_artifact_error, render_execution_failure};

const README_FIXTURE_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>HTMLCut README Fixture</title>
</head>
<body>
  <main>
    <article>
      <h1>Guide</h1>
      <div class="card">Card alpha</div>
      <div class="card">Card beta</div>
      <p><a class="more" href="../guide.html">Read more</a></p>
      <pre>START::Regex slice payload::END</pre>
    </article>
  </main>
</body>
</html>
"#;

const SANDBOX_FIXTURE_FILES: &[&str] = &["page.html", "page name.html"];

pub(super) fn prepare_sandbox(
    display_path: &str,
) -> Result<(ExampleSandbox, CurrentDirGuard), Vec<String>> {
    prepare_sandbox_with_hooks(
        display_path,
        || ExampleSandbox::new().map_err(|error| error.to_string()),
        |sandbox| sandbox.enter().map_err(|error| error.to_string()),
    )
}

pub(super) fn prepare_sandbox_with_hooks<NewSandbox, EnterSandbox>(
    display_path: &str,
    new_sandbox: NewSandbox,
    enter_sandbox: EnterSandbox,
) -> Result<(ExampleSandbox, CurrentDirGuard), Vec<String>>
where
    NewSandbox: FnOnce() -> Result<ExampleSandbox, String>,
    EnterSandbox: FnOnce(&ExampleSandbox) -> Result<CurrentDirGuard, String>,
{
    let sandbox = match new_sandbox() {
        Ok(sandbox) => sandbox,
        Err(error) => {
            return Err(vec![format!(
                "{display_path} could not initialize the htmlcut docs-example sandbox: {error}"
            )]);
        }
    };
    let guard = match enter_sandbox(&sandbox) {
        Ok(guard) => guard,
        Err(error) => {
            return Err(vec![format!(
                "{display_path} could not enter the htmlcut docs-example sandbox: {error}"
            )]);
        }
    };

    Ok((sandbox, guard))
}

pub(super) struct ExampleSandbox {
    root: TempDir,
}

impl ExampleSandbox {
    pub(super) fn new() -> DynResult<Self> {
        let sandbox = Self { root: tempdir()? };
        sandbox.seed()?;
        Ok(sandbox)
    }

    pub(super) fn enter(&self) -> DynResult<CurrentDirGuard> {
        CurrentDirGuard::enter(self.root.path())
    }

    fn seed(&self) -> DynResult<()> {
        for file_name in SANDBOX_FIXTURE_FILES {
            fs::write(self.root.path().join(file_name), README_FIXTURE_HTML)?;
        }

        Ok(())
    }

    pub(super) fn command_runtime_error(
        &self,
        display_path: &str,
        example: &str,
        tokens: &[String],
    ) -> Option<String> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(error) = command_runtime_error_message(
            display_path,
            example,
            htmlcut_cli::run(tokens.iter().cloned(), &mut stdout, &mut stderr),
            &stdout,
            &stderr,
        ) {
            return Some(error);
        }

        documented_artifact_error(display_path, example, tokens)
    }
}

fn command_runtime_error_message(
    display_path: &str,
    example: &str,
    result: io::Result<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<String> {
    match result {
        Ok(0) => None,
        Ok(exit_code) => Some(format!(
            "{display_path} contains a non-runnable htmlcut example: {example} ({})",
            render_execution_failure(exit_code, stdout, stderr)
        )),
        Err(error) => Some(format!(
            "{display_path} contains a non-runnable htmlcut example: {example} (failed to capture CLI output: {error})"
        )),
    }
}

#[cfg(test)]
pub(crate) fn command_runtime_error_message_for_tests(
    display_path: &str,
    example: &str,
    result: io::Result<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<String> {
    command_runtime_error_message(display_path, example, result, stdout, stderr)
}

fn current_dir_lock() -> &'static Mutex<()> {
    static CURRENT_DIR_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    CURRENT_DIR_LOCK.get_or_init(|| Mutex::new(()))
}

pub(super) struct CurrentDirGuard {
    _lock: MutexGuard<'static, ()>,
    previous_dir: PathBuf,
}

impl CurrentDirGuard {
    fn enter(dir: &Path) -> DynResult<Self> {
        let lock = current_dir_lock()
            .lock()
            .map_err(|_| "cwd mutex poisoned".to_owned())?;
        let previous_dir = env::current_dir()?;
        env::set_current_dir(dir)?;

        Ok(Self {
            _lock: lock,
            previous_dir,
        })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.previous_dir);
    }
}
