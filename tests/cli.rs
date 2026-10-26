use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_payments-engine");

fn scratch(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn write_input(name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let path = scratch(name);
    fs::write(&path, bytes)?;
    Ok(path)
}

fn run_bin(args: &[&OsStr]) -> io::Result<Output> {
    Command::new(BIN).args(args).stdin(Stdio::null()).output()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn check_eq<T: PartialEq + Debug>(failures: &mut Vec<String>, what: &str, got: T, want: T) {
    if got != want {
        failures.push(format!("{what}: got {got:?}, want {want:?}"));
    }
}

fn check(failures: &mut Vec<String>, what: &str, ok: bool) {
    if !ok {
        failures.push(format!("{what}: condition failed"));
    }
}

#[test]
fn success_prints_accounts_only() -> io::Result<()> {
    let path = write_input(
        "success_prints_accounts_only.csv",
        b"type,client,tx,amount\ndeposit,1,1,1.5\n",
    )?;
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(
        &mut failures,
        "stdout",
        stdout.as_str(),
        "client,available,held,total,locked\n1,1.5000,0.0000,1.5000,false\n",
    );
    check_eq(&mut failures, "stderr", stderr.as_str(), "");
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn bad_row_logs_to_stderr_not_stdout() -> io::Result<()> {
    let path = write_input(
        "bad_row_logs_to_stderr_not_stdout.csv",
        b"type,client,tx,amount\ndeposit,1,1,10\ndeposit,1,2,abc\n",
    )?;
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(
        &mut failures,
        "stdout",
        stdout.as_str(),
        "client,available,held,total,locked\n1,10.0000,0.0000,10.0000,false\n",
    );
    check(
        &mut failures,
        "stderr has a line starting `bad: `",
        stderr.lines().any(|line| line.starts_with("bad: ")),
    );
    check(
        &mut failures,
        "stderr has the line `skipped bad: 1`",
        stderr.lines().any(|line| line == "skipped bad: 1"),
    );
    check(&mut failures, "stdout has no `bad`", !stdout.contains("bad"));
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn missing_file_exits_1() -> io::Result<()> {
    let path = scratch("missing_file_exits_1.csv");
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(1));
    check_eq(&mut failures, "stdout", stdout.as_str(), "");
    check(&mut failures, "stderr is non-empty", !stderr.is_empty());
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn no_argument_exits_2() -> io::Result<()> {
    let out = run_bin(&[])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(2));
    check_eq(&mut failures, "stdout", stdout.as_str(), "");
    check(&mut failures, "stderr is non-empty (usage)", !stderr.is_empty());
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn two_arguments_exit_2() -> io::Result<()> {
    let first =
        write_input("two_arguments_exit_2_a.csv", b"type,client,tx,amount\ndeposit,1,1,1\n")?;
    let second =
        write_input("two_arguments_exit_2_b.csv", b"type,client,tx,amount\ndeposit,2,2,2\n")?;
    let out = run_bin(&[first.as_os_str(), second.as_os_str()])?;
    let stdout = text(&out.stdout);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(2));
    check_eq(&mut failures, "stdout", stdout.as_str(), "");
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn directory_as_input_exits_1_with_no_output() -> io::Result<()> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let out = run_bin(&[dir.as_os_str()])?;
    let stdout = text(&out.stdout);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(1));
    check_eq(&mut failures, "stdout", stdout.as_str(), "");
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn log_cap_is_100() -> io::Result<()> {
    let mut text_in = String::from("type,client,tx,amount\n");
    for _ in 0..150 {
        text_in.push_str("foo,1,1,1\n");
    }
    let path = write_input("log_cap_is_100.csv", text_in.as_bytes())?;
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "stderr line count", lines.len(), 101);
    check_eq(
        &mut failures,
        "stderr lines starting `bad: `",
        lines.iter().filter(|line| line.starts_with("bad: ")).count(),
        100,
    );
    check(
        &mut failures,
        "stderr has the line `skipped bad: 150`",
        lines.contains(&"skipped bad: 150"),
    );
    check_eq(&mut failures, "stdout", stdout.as_str(), "client,available,held,total,locked\n");
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn bom_header_is_not_a_warning() -> io::Result<()> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"type,client,tx,amount\ndeposit,1,1,1\n");
    let path = write_input("bom_header_is_not_a_warning.csv", &bytes)?;
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "stderr", stderr.as_str(), "");
    check_eq(
        &mut failures,
        "stdout",
        stdout.as_str(),
        "client,available,held,total,locked\n1,1.0000,0.0000,1.0000,false\n",
    );
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn missing_header_warns_once() -> io::Result<()> {
    let path = write_input("missing_header_warns_once.csv", b"deposit,1,1,10\n")?;
    let out = run_bin(&[path.as_os_str()])?;
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "stderr line count", lines.len(), 1);
    check(
        &mut failures,
        "stderr line starts with `warning:`",
        lines.first().is_some_and(|line| line.starts_with("warning:")),
    );
    check_eq(
        &mut failures,
        "stdout",
        stdout.as_str(),
        "client,available,held,total,locked\n1,10.0000,0.0000,10.0000,false\n",
    );
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn closed_stdout_exits_1_without_panic() -> io::Result<()> {
    let mut text_in = String::from("type,client,tx,amount\n");
    for n in 1..=20000 {
        text_in.push_str(&format!("deposit,{n},{n},1\n"));
    }
    let path = write_input("closed_stdout_exits_1_without_panic.csv", text_in.as_bytes())?;
    let mut child =
        Command::new(BIN).arg(&path).stdin(Stdio::null()).stdout(Stdio::piped()).spawn()?;
    drop(child.stdout.take());
    let status = child.wait()?;
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", status.code(), Some(1));
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn closed_stderr_does_not_change_exit() -> io::Result<()> {
    let path = write_input(
        "closed_stderr_does_not_change_exit.csv",
        b"type,client,tx,amount\ndeposit,1,1,abc\ndeposit,1,2,5\n",
    )?;
    let out = Command::new("sh")
        .arg("-c")
        .arg("exec \"$0\" \"$1\" 2>&-")
        .arg(BIN)
        .arg(&path)
        .stdin(Stdio::null())
        .output()?;
    let stdout = text(&out.stdout);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(
        &mut failures,
        "stdout",
        stdout.as_str(),
        "client,available,held,total,locked\n1,5.0000,0.0000,5.0000,false\n",
    );
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}
