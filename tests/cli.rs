use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sql-tdg-{name}-{}-{counter}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test directory should be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn compiled_cli_generates_parquet_and_metadata_from_inline_sql() {
    let workspace = TestDir::new("inline");
    let output_dir = workspace.path().join("generated");

    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--dialect",
            "generic",
            "--sql",
            "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
            "--schema",
            "orders:amount=INTEGER",
            "--matching",
            "3",
            "--rejected",
            "2",
            "--seed",
            "17",
            "--format",
            "parquet",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_dir.join("0001-orders.parquet").is_file());
    assert!(output_dir.join("metadata.sqltdg").is_file());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("target=anonymous:layer-0001"));
    assert!(stdout.contains("matching_rows=3"));
    assert!(stdout.contains("rejected_rows=2"));
    assert!(stdout.contains("format=parquet"));
    assert!(stdout.contains("relation=orders"));
}

#[test]
fn compiled_cli_reads_sql_files_and_exports_csv() {
    let workspace = TestDir::new("file");
    let query_path = workspace.path().join("query.sql");
    let output_dir = workspace.path().join("generated");
    fs::write(
        &query_path,
        "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
    )
    .expect("query fixture should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--dialect",
            "generic",
            "--file",
        ])
        .arg(&query_path)
        .args([
            "--schema",
            "orders:amount=INTEGER",
            "--matching",
            "2",
            "--format",
            "csv",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let csv_path = output_dir.join("0001-orders.csv");
    assert!(csv_path.is_file());
    let csv = fs::read_to_string(csv_path).expect("CSV output should be readable");
    assert!(csv.starts_with("amount\n"));
}

#[test]
fn compiled_cli_returns_nonzero_for_missing_raw_schema() {
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--sql",
            "SELECT amount FROM orders WHERE amount >= 10",
        ])
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("requires at least one --schema"));
}
