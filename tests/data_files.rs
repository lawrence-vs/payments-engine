use std::fs;
use std::path::{Path, PathBuf};

const PAIR_COUNT: usize = 21;

#[test]
fn data_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data");
    let mut inputs: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("tests/data is readable")
        .map(|entry| entry.expect("tests/data entry is readable").path())
        .filter(|path| path.to_string_lossy().ends_with(".in.csv"))
        .collect();
    inputs.sort();
    let mut failures = Vec::new();
    if inputs.len() != PAIR_COUNT {
        failures.push(format!("found {} .in.csv files, want {PAIR_COUNT}", inputs.len()));
    }
    for input_path in &inputs {
        let name = input_path.to_string_lossy().into_owned();
        let Some(base) = name.strip_suffix(".in.csv") else {
            failures.push(format!("{name}: not an .in.csv file"));
            continue;
        };
        let output_path = format!("{base}.out.csv");
        let want = match fs::read(&output_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{name}: no expected output {output_path}: {error}"));
                continue;
            }
        };
        let input = match fs::read(input_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{name}: cannot read input: {error}"));
                continue;
            }
        };
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        if let Err(error) = payments_engine::run(input.as_slice(), &mut out, &mut err) {
            failures.push(format!("{name}: run failed: {error}"));
            continue;
        }
        if out != want {
            failures.push(format!(
                "{name}: stdout {:?}, want {:?}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&want)
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
