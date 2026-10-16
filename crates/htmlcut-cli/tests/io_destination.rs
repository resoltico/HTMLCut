// SPDX-License-Identifier: MPL-2.0
//! Native destination boundaries exercised through the built or delivered binary.
#![cfg(unix)]

// This suite uses preloaded pipe input; the other modes are used by sibling suites.
#[allow(dead_code)]
#[path = "support/capture.rs"]
mod capture;

use std::{
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{FileTypeExt, symlink},
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}

fn command(source: &Path, destination: &Path) -> Command {
    let mut command = Command::new(binary());
    command
        .args(["get", "h1"])
        .arg(source)
        .arg("-o")
        .arg(destination)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn complete(mut command: Command) -> Output {
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("destination publication did not finish: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn descriptor_link_chains_obey_kernel_admission_without_changing_stream_state() {
    use std::fs::OpenOptions;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let mut aliases = vec!["/dev/stdout", "/dev/fd/1"];
    if cfg!(target_os = "linux") {
        aliases.extend(["/proc/self/fd/1", "/proc/thread-self/fd/1"]);
    }
    for (index, alias) in aliases.iter().enumerate() {
        let links = directory.path().join(index.to_string());
        std::fs::create_dir(&links).unwrap();
        let mut destination = std::path::PathBuf::from(alias);
        let mut accepted = 0;
        let mut refused = 0;
        for count in 1..=64 {
            let link = links.join(count.to_string());
            symlink(&destination, &link).unwrap();
            destination = link;
            let target = directory.path().join("output");
            std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
            let mut stream = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&target)
                .unwrap();
            stream.seek(SeekFrom::Start(7)).unwrap();
            let oracle = output_native_oracle(&destination, &stream);
            assert_eq!(oracle[0], oracle[1], "{alias}: {count}");
            assert_eq!(stream.stream_position().unwrap(), 7);
            assert_eq!(std::fs::read(&target).unwrap(), b"prefix|XXXXXXXXXXXXXXXX");
            if oracle[0] != 0 {
                assert_eq!(oracle[0], libc::ELOOP);
            }
            if count == 1 || oracle[0] != 0 {
                for append in [false, true] {
                    std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
                    let mut stream = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .append(append)
                        .open(&target)
                        .unwrap();
                    stream.seek(SeekFrom::Start(7)).unwrap();
                    let mut invocation = command(&source, &destination);
                    invocation.stdout(stream.try_clone().unwrap());
                    let output = complete(invocation);
                    if oracle[0] == 0 {
                        assert!(output.status.success(), "{output:?}");
                        assert!(output.stderr.is_empty());
                        assert_eq!(
                            std::fs::read(&target).unwrap(),
                            if append {
                                &b"prefix|XXXXXXXXXXXXXXXX\"complete\"\n"[..]
                            } else {
                                &b"prefix|\"complete\"\nXXXXX"[..]
                            }
                        );
                        assert_eq!(
                            stream.stream_position().unwrap(),
                            if append { 34 } else { 18 }
                        );
                        accepted += 1;
                    } else {
                        assert_eq!(output.status.code(), Some(5), "{output:?}");
                        let error: serde_json::Value =
                            serde_json::from_slice(&output.stderr).unwrap();
                        assert_eq!(error["error"], "output_unwritable");
                        assert_eq!(error["file"], destination.to_str().unwrap());
                        assert_eq!(stream.stream_position().unwrap(), 7);
                        assert_eq!(std::fs::read(&target).unwrap(), b"prefix|XXXXXXXXXXXXXXXX");
                        refused += 1;
                    }
                }
                if refused > 0 {
                    break;
                }
            }
        }
        assert_eq!((accepted, refused), (2, 2));
    }
}
// After fork this probe uses only async-signal-safe syscalls, with all buffers
// allocated beforehand. It exits before exec; no test-harness output reaches fd1.
#[allow(unsafe_code)]
fn output_native_oracle(path: &Path, stream: &std::fs::File) -> [i32; 2] {
    use std::os::unix::{ffi::OsStrExt, process::CommandExt};
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    let mut invocation = Command::new(binary());
    invocation
        .stdout(stream.try_clone().unwrap())
        .stderr(Stdio::piped());
    unsafe {
        invocation.pre_exec(move || {
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let first = if libc::stat(path.as_ptr(), stat.as_mut_ptr()) == 0 {
                0
            } else {
                native_errno()
            };
            let fd = libc::open(path.as_ptr(), libc::O_WRONLY);
            let second = if fd >= 0 {
                libc::close(fd);
                0
            } else {
                native_errno()
            };
            let answers = [first, second];
            libc::write(2, answers.as_ptr().cast(), std::mem::size_of_val(&answers));
            libc::_exit(0);
        });
    }
    let result = invocation.output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stderr.len(), 8);
    [
        i32::from_ne_bytes(result.stderr[..4].try_into().unwrap()),
        i32::from_ne_bytes(result.stderr[4..].try_into().unwrap()),
    ]
}
#[allow(unsafe_code)]
unsafe fn native_errno() -> i32 {
    #[cfg(target_os = "macos")]
    {
        unsafe { *libc::__error() }
    }
    #[cfg(target_os = "linux")]
    {
        unsafe { *libc::__errno_location() }
    }
}

#[test]
fn descriptor_directory_suffixes_remain_input_errors_with_contract_stdin() {
    let mut paths = vec!["/dev/fd/0/", "/dev/fd/0/."];
    if cfg!(target_os = "linux") {
        paths.extend([
            "/proc/self/fd/0/",
            "/proc/self/fd/0/.",
            "/proc/thread-self/fd/0/",
            "/proc/thread-self/fd/0/.",
        ]);
    }
    for path in paths {
        let mut invocation = Command::new(binary());
        invocation.args(["run", "--from-file", "-", path]);
        let result = capture::finite_capture(
            &mut invocation,
            Duration::from_secs(3),
            1024 * 1024,
            capture::Stdin::Bytes(b"{\"select\":\"h1\"}"),
        )
        .unwrap();
        assert_eq!(result.status.code(), Some(5), "{result:?}");
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"], "input_unreadable");
        assert_eq!(error["file"], path);
        let detail = error["detail"].as_str().unwrap();
        assert!(
            detail.contains(&format!("(os error {})", libc::ENOTDIR))
                || detail.contains(&format!("(os error {})", libc::ENOENT)),
            "{detail}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn output_accepts_the_os_symlink_limit_and_refuses_one_more_link() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    for existing in [false, true] {
        let root = directory
            .path()
            .join(if existing { "existing" } else { "absent" });
        std::fs::create_dir(&root).unwrap();
        let target = root.join("target");
        if existing {
            std::fs::write(&target, b"old").unwrap();
        }
        let mut destination = target.clone();
        for index in 0..40 {
            let link = root.join(format!("link-{index}"));
            symlink(destination.file_name().unwrap(), &link).unwrap();
            destination = link;
        }
        let output = complete(command(&source, &destination));
        assert!(output.status.success(), "{output:?}");
        assert_eq!(std::fs::read(&target).unwrap(), b"\"complete\"\n");
        let extra = root.join("extra-link");
        symlink(destination.file_name().unwrap(), &extra).unwrap();
        let output = complete(command(&source, &extra));
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        assert_eq!(std::fs::read(&target).unwrap(), b"\"complete\"\n");
        assert!(
            destination
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 42);
    }
}

#[test]
fn output_through_a_symlinked_descriptor_parent_reaches_the_open_file() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let target = directory.path().join("redirected");
    let aliases = directory.path().join("descriptors");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    symlink("/dev/fd", &aliases).unwrap();
    let writer = std::fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&target)
        .unwrap();
    let mut observer = writer.try_clone().unwrap();
    let mut publication = command(&source, &aliases.join("1"));
    publication.stdout(Stdio::from(writer));
    let output = complete(publication);
    assert!(output.status.success(), "{output:?}");
    observer.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    observer.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"\"complete\"\n");
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert!(aliases.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn output_fifo_reader_disconnect_is_silent_success() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let pipe = directory.path().join("output.pipe");
    std::fs::write(&source, format!("<h1>{}</h1>", "x".repeat(1024 * 1024))).unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    let reader_path = pipe.clone();
    let reader = std::thread::spawn(move || {
        let mut reader = std::fs::File::open(reader_path).unwrap();
        let mut byte = [0];
        reader.read_exact(&mut byte).unwrap();
        byte
    });
    let output = complete(command(&source, &pipe));
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(reader.join().unwrap(), [b'"']);
    assert!(pipe.symlink_metadata().unwrap().file_type().is_fifo());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn relative_output_names_create_and_follow_a_relative_link() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let mut publication = command(&source, Path::new("result.json"));
    publication.current_dir(directory.path());
    let output = complete(publication);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read(directory.path().join("result.json")).unwrap(),
        b"\"complete\"\n"
    );
    symlink("result.json", directory.path().join("alias")).unwrap();
    std::fs::write(&source, b"<h1>changed</h1>").unwrap();
    let mut publication = command(&source, Path::new("alias"));
    publication.current_dir(directory.path());
    let output = complete(publication);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read(directory.path().join("result.json")).unwrap(),
        b"\"changed\"\n"
    );
    assert!(
        directory
            .path()
            .join("alias")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
fn output_stages_in_its_writable_directory_when_the_working_directory_is_read_only() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let working = directory.path().join("working");
    let destinations = working.join("output");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    std::fs::create_dir(&working).unwrap();
    std::fs::create_dir(&destinations).unwrap();
    std::fs::write(destinations.join("existing.json"), b"old").unwrap();
    let permissions = working.metadata().unwrap().permissions();
    std::fs::set_permissions(&working, std::fs::Permissions::from_mode(0o555)).unwrap();
    let operation = std::panic::catch_unwind(|| {
        let refusal = std::fs::write(working.join("must-refuse"), b"control");
        let outputs = ["output/created.json", "output/existing.json"].map(|path| {
            let mut publication = command(&source, Path::new(path));
            publication.current_dir(&working);
            complete(publication)
        });
        (refusal, outputs)
    });
    // Restore before assertions or resuming a child-process failure, so cleanup works.
    std::fs::set_permissions(&working, permissions).unwrap();
    let (refusal, outputs) = operation.unwrap();
    let error = refusal.expect_err("the test requires effective directory write permissions");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    for output in outputs {
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    for name in ["created.json", "existing.json"] {
        assert_eq!(
            std::fs::read(destinations.join(name)).unwrap(),
            b"\"complete\"\n"
        );
    }
    assert_eq!(std::fs::read_dir(&working).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(&destinations).unwrap().count(), 2);
}
