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
        let path =
            std::env::temp_dir().join(format!("sql-tdg-{name}-{}-{counter}", std::process::id()));
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
        .args(["generate", "--dialect", "generic", "--file"])
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

fn run_all_outcomes(workspace: &TestDir, sql: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--sql",
            sql,
            "--schema",
            "orders:amount=INTEGER",
            "--schema",
            "customers:id=INTEGER",
            "--matching",
            "6",
            "--format",
            "csv",
            "--output",
        ])
        .arg(workspace.path().join("generated"))
        .output()
        .expect("compiled sql-tdg binary should execute")
}

#[test]
fn compiled_cli_generates_one_shared_dataset_for_all_terminal_outcomes() {
    let workspace = TestDir::new("all-outcomes");
    let output = run_all_outcomes(
        &workspace,
        "CREATE VIEW high AS SELECT amount FROM orders WHERE amount >= 10;
         CREATE VIEW capped AS SELECT amount FROM orders WHERE amount <= 20;
         CREATE VIEW vip AS SELECT id FROM customers WHERE id > 100;",
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.contains("target=all:relation:capped, relation:high, relation:vip"),
        "stdout: {stdout}"
    );

    let generated = workspace.path().join("generated");
    let amounts = fs::read_to_string(generated.join("0002-orders.csv"))
        .expect("shared orders source should be exported once");
    let amounts = amounts
        .lines()
        .skip(1)
        .map(|line| line.parse::<i32>().expect("amount should be an integer"))
        .collect::<Vec<_>>();
    assert_eq!(amounts.len(), 6);
    assert!(amounts.iter().all(|value| (10..=20).contains(value)));
    assert!(generated.join("0001-customers.csv").is_file());

    let metadata =
        fs::read_to_string(generated.join("metadata.sqltdg")).expect("metadata should be readable");
    let metadata =
        sql_tdg::TestCaseMetadata::deserialize(&metadata).expect("metadata should round-trip");
    assert_eq!(
        metadata.target().kind(),
        sql_tdg::TargetKind::AllTerminalOutcomes
    );
}

#[test]
fn compiled_cli_rejects_conflicting_terminal_outcomes_without_target() {
    let workspace = TestDir::new("all-outcomes-conflict");
    let output = run_all_outcomes(
        &workspace,
        "CREATE VIEW big AS SELECT amount FROM orders WHERE amount > 100;
         CREATE VIEW small AS SELECT amount FROM orders WHERE amount < 50;",
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        stderr.contains("relation:big, relation:small") && stderr.contains("orders.amount"),
        "stderr: {stderr}"
    );
}

#[test]
fn compiled_cli_explains_dbt_sources_missing_from_catalog() {
    let workspace = TestDir::new("dbt-empty-catalog");
    let manifest_path = workspace.path().join("manifest.json");
    let catalog_path = workspace.path().join("catalog.json");
    fs::write(
        &manifest_path,
        r#"{
            "metadata": {
                "dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json",
                "adapter_type": "duckdb"
            },
            "nodes": {
                "model.demo.daily_revenue": {
                    "unique_id": "model.demo.daily_revenue",
                    "resource_type": "model",
                    "relation_name": "\"warehouse\".\"main\".\"daily_revenue\"",
                    "language": "sql",
                    "compiled_code": "select amount from \"warehouse\".\"main\".\"orders\" where amount > 10",
                    "depends_on": {"nodes": ["source.demo.raw.orders"]},
                    "database": "warehouse",
                    "schema": "main"
                }
            },
            "sources": {
                "source.demo.raw.orders": {
                    "unique_id": "source.demo.raw.orders",
                    "relation_name": "\"warehouse\".\"main\".\"orders\""
                }
            }
        }"#,
    )
    .expect("manifest fixture should be writable");
    fs::write(
        &catalog_path,
        r#"{
            "metadata": {
                "dbt_schema_version": "https://schemas.getdbt.com/dbt/catalog/v1.json",
                "dbt_version": "1.12.3"
            },
            "nodes": {},
            "sources": {},
            "errors": null
        }"#,
    )
    .expect("catalog fixture should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args(["generate", "--dbt-manifest"])
        .arg(&manifest_path)
        .arg("--output")
        .arg(workspace.path().join("generated"))
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("orders"), "stderr: {stderr}");
    assert!(
        stderr.contains("every source table must exist")
            && stderr.contains("rerun `dbt docs generate`"),
        "stderr: {stderr}"
    );
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

#[test]
fn compiled_cli_requires_and_records_comparison_assumptions() {
    let workspace = TestDir::new("comparison-assumptions");
    let without = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--sql",
            "SELECT name FROM customers WHERE name = 'Alice'",
            "--schema",
            "customers:name=VARCHAR",
            "--output",
        ])
        .arg(workspace.path().join("undeclared"))
        .output()
        .expect("compiled CLI must execute");
    assert!(!without.status.success());
    assert!(
        String::from_utf8_lossy(&without.stderr).contains("binary_collation"),
        "stderr: {}",
        String::from_utf8_lossy(&without.stderr)
    );

    let output_dir = workspace.path().join("declared");
    let with = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--sql",
            "SELECT name FROM customers WHERE name = 'Alice'",
            "--schema",
            "customers:name=VARCHAR",
            "--assume-comparison",
            "binary_collation",
            "--matching",
            "3",
            "--format",
            "csv",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled CLI must execute");
    assert!(
        with.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&with.stderr)
    );
    let metadata =
        fs::read_to_string(output_dir.join("metadata.sqltdg")).expect("metadata must be written");
    let metadata =
        sql_tdg::TestCaseMetadata::deserialize(&metadata).expect("metadata must round-trip");
    assert!(
        metadata.protocol().document().contains("binary_collation"),
        "protocol snapshot must retain comparison declarations"
    );
}
