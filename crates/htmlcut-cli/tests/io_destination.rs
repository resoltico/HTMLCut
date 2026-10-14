// SPDX-License-Identifier: MPL-2.0
//! Native destination boundaries exercised through the built or delivered binary.
#![cfg(unix)]

use std::{
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{FileTypeExt, symlink},
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn command(source: &Path, destination: &Path) -> Command {
    let binary = std::env::var_os("HTMLCUT_TEST_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into());
    let mut command = Command::new(binary);
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
    let directory = tempfile::tempdir().unwrap();
    let binary = std::env::var_os("HTMLCUT_TEST_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into());
    let output = Command::new("python3").args(["-c", r#"
import errno, json, pathlib, subprocess, sys
binary, workspace = sys.argv[1:]
root = pathlib.Path(workspace)
source = root / 'page.html'
source.write_bytes(b'<h1>complete</h1>')
aliases = ['/dev/stdout', '/dev/fd/1']
if sys.platform.startswith('linux'):
    aliases += ['/proc/self/fd/1', '/proc/thread-self/fd/1']
oracle = '''import json, os, sys
path = sys.argv[1]
answers = []
for operation in [lambda: os.stat(path), lambda: os.close(os.open(path, os.O_WRONLY))]:
    try:
        operation()
        answers.append(None)
    except OSError as error:
        answers.append(error.errno)
sys.stderr.write(json.dumps(answers))
'''
for index, alias in enumerate(aliases):
    links = root / str(index)
    links.mkdir()
    destination = pathlib.Path(alias)
    accepted = 0
    refused = 0
    for count in range(1, 65):
        link = links / str(count)
        link.symlink_to(destination)
        destination = link
        target = root / 'output'
        target.write_bytes(b'prefix|XXXXXXXXXXXXXXXX')
        with target.open('r+b', buffering=0) as stream:
            stream.seek(7)
            control = subprocess.run([sys.executable, '-c', oracle, str(destination)],
                stdout=stream, stderr=subprocess.PIPE, timeout=10)
            assert control.returncode == 0, control
            outcomes = json.loads(control.stderr)
            assert outcomes[0] == outcomes[1], (alias, count, outcomes)
            assert stream.tell() == 7 and target.read_bytes() == b'prefix|XXXXXXXXXXXXXXXX'
        if outcomes[0] is not None:
            assert outcomes[0] == errno.ELOOP, (alias, count, outcomes)
        # Exercise the shortest accepted chain and both sides of the actual
        # kernel boundary; no namespace's magic links are guessed or counted.
        if count == 1 or outcomes[0] is not None:
            for append in [False, True]:
                target.write_bytes(b'prefix|XXXXXXXXXXXXXXXX')
                with target.open('ab' if append else 'r+b', buffering=0) as stream:
                    stream.seek(7)
                    result = subprocess.run([binary, 'get', 'h1', str(source), '-o', str(destination)],
                        stdout=stream, stderr=subprocess.PIPE, timeout=10)
                    if outcomes[0] is None:
                        assert result.returncode == 0 and not result.stderr, result
                        expected = b'prefix|XXXXXXXXXXXXXXXX"complete"\n' if append else b'prefix|"complete"\nXXXXX'
                        assert target.read_bytes() == expected
                        assert stream.tell() == (34 if append else 18)
                        accepted += 1
                    else:
                        assert result.returncode == 5, (alias, count, result)
                        error = json.loads(result.stderr)
                        assert error['error'] == 'output_unwritable'
                        assert error['file'] == str(destination)
                        assert error['detail']
                        assert target.read_bytes() == b'prefix|XXXXXXXXXXXXXXXX'
                        assert stream.tell() == 7
                        refused += 1
            if refused:
                break
    assert accepted == 2 and refused == 2, (alias, accepted, refused)
"#]).arg(binary).arg(directory.path()).output().unwrap();
    assert!(output.status.success(), "{output:?}");
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
