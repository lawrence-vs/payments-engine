use std::io;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const HEADER: &str = "type,client,tx,amount";

fn example_path() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let deps = exe.parent().ok_or_else(|| io::Error::other("test exe has no deps dir"))?;
    let profile = deps.parent().ok_or_else(|| io::Error::other("deps dir has no profile dir"))?;
    let path = profile.join("examples").join(format!("gen{}", std::env::consts::EXE_SUFFIX));
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "build the examples first: cargo test builds them (missing {})",
                path.display()
            ),
        ));
    }
    Ok(path)
}

fn run_gen(args: &[&str]) -> io::Result<Output> {
    Command::new(example_path()?).args(args).stdin(Stdio::null()).output()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn check_eq<T: PartialEq + std::fmt::Debug>(
    failures: &mut Vec<String>,
    what: &str,
    got: T,
    want: T,
) {
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
fn same_seed_same_bytes() -> io::Result<()> {
    let first = run_gen(&["5000", "7"])?;
    let second = run_gen(&["5000", "7"])?;
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", first.status.code(), Some(0));
    check(&mut failures, "stdout is non-empty", !first.stdout.is_empty());
    check(&mut failures, "stdout identical across runs", first.stdout == second.stdout);
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn different_seed_differs() -> io::Result<()> {
    let seven = run_gen(&["5000", "7"])?;
    let eight = run_gen(&["5000", "8"])?;
    let mut failures = Vec::new();
    check(
        &mut failures,
        "seed 7 and seed 8 produce different stdout",
        seven.stdout != eight.stdout,
    );
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn header_and_row_count() -> io::Result<()> {
    let out = run_gen(&["1000", "3"])?;
    let stdout = text(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "first line", lines.first().copied(), Some(HEADER));
    check_eq(&mut failures, "line count", lines.len(), 1001);
    check(&mut failures, "stdout ends with newline", stdout.ends_with('\n'));
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn default_args_work() -> io::Result<()> {
    let out = run_gen(&["10"])?;
    let stdout = text(&out.stdout);
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "line count", stdout.lines().count(), 11);
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

struct BadCase {
    name: &'static str,
    args: &'static [&'static str],
}

const BAD_CASES: &[BadCase] = &[
    BadCase { name: "no args", args: &[] },
    BadCase { name: "non-numeric rows", args: &["abc"] },
    BadCase { name: "non-numeric seed", args: &["10", "xyz"] },
    BadCase { name: "zero clients", args: &["10", "1", "0"] },
    BadCase { name: "clients above u16", args: &["10", "1", "70000"] },
    BadCase { name: "four arguments", args: &["10", "1", "5", "extra"] },
];

#[test]
fn bad_args_exit_2() -> io::Result<()> {
    let mut failures = Vec::new();
    for case in BAD_CASES {
        let out = run_gen(case.args)?;
        check_eq(&mut failures, &format!("{}: exit code", case.name), out.status.code(), Some(2));
        check(&mut failures, &format!("{}: stdout is empty", case.name), out.stdout.is_empty());
    }
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn clients_bounded() -> io::Result<()> {
    let out = run_gen(&["5000", "1", "5"])?;
    let stdout = text(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    check_eq(&mut failures, "line count", lines.len(), 5001);
    for line in lines.iter().skip(1) {
        let client = line.split(',').nth(1).and_then(|f| f.parse::<u16>().ok());
        check(
            &mut failures,
            &format!("client in 1..=5 on `{line}`"),
            client.is_some_and(|c| (1..=5).contains(&c)),
        );
    }
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn output_is_valid_input() -> io::Result<()> {
    let out = run_gen(&["20000", "1", "50"])?;
    let mut accounts = Vec::new();
    let mut err = Vec::new();
    let result = payments_engine::run(&out.stdout[..], &mut accounts, &mut err);
    let accounts_text = text(&accounts);
    let mut failures = Vec::new();
    check_eq(&mut failures, "gen exit code", out.status.code(), Some(0));
    if let Err(e) = &result {
        failures.push(format!("run returned error: {e}"));
    }
    check(
        &mut failures,
        "accounts start with header",
        accounts_text.starts_with("client,available,held,total,locked\n"),
    );
    let account_lines: Vec<&str> = accounts_text.lines().skip(1).collect();
    check(&mut failures, "at least one account line", !account_lines.is_empty());
    check(&mut failures, "at most 50 account lines", account_lines.len() <= 50);
    for line in &account_lines {
        let id = line.split(',').next().and_then(|f| f.parse::<u16>().ok());
        check(
            &mut failures,
            &format!("account id in 1..=50 on `{line}`"),
            id.is_some_and(|c| (1..=50).contains(&c)),
        );
    }
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn mix_is_exercised() -> io::Result<()> {
    let out = run_gen(&["100000", "1", "1000"])?;
    let stdout = text(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", out.status.code(), Some(0));
    for prefix in ["deposit,", "withdrawal,", "dispute,", "resolve,", "chargeback,", "foo,"] {
        check(
            &mut failures,
            &format!("has a line starting `{prefix}`"),
            lines.iter().any(|line| line.starts_with(prefix)),
        );
    }
    check(
        &mut failures,
        "has a three-field dispute line",
        lines.iter().any(|line| line.starts_with("dispute,") && line.matches(',').count() == 2),
    );
    check(
        &mut failures,
        "has a four-field dispute line ending in a comma",
        lines.iter().any(|line| line.starts_with("dispute,") && line.ends_with(',')),
    );
    check(
        &mut failures,
        "has a deposit line ending `,0`",
        lines.iter().any(|line| line.starts_with("deposit,") && line.ends_with(",0")),
    );
    check(
        &mut failures,
        "has a deposit line ending `,abc`",
        lines.iter().any(|line| line.starts_with("deposit,") && line.ends_with(",abc")),
    );
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
fn closed_pipe_exits_1_without_panic() -> io::Result<()> {
    let mut child = Command::new(example_path()?)
        .args(["20000000", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    drop(child.stdout.take());
    let status = child.wait()?;
    let mut failures = Vec::new();
    check_eq(&mut failures, "exit code", status.code(), Some(1));
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}

#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn ten_million_rows_stream_through_run() -> io::Result<()> {
    let mut child = Command::new(example_path()?)
        .args(["10000000", "1", "1000"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| io::Error::other("child has no stdout pipe"))?;
    let result = payments_engine::run(stdout, io::sink(), io::sink());
    let status = child.wait()?;
    let mut failures = Vec::new();
    if let Err(e) = &result {
        failures.push(format!("run returned error: {e}"));
    }
    check_eq(&mut failures, "gen exit code", status.code(), Some(0));
    assert!(failures.is_empty(), "{failures:#?}");
    Ok(())
}
