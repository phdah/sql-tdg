//! Smoke tests for the copyable examples in README.md and docs/usage.md.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use sql_tdg::{RelationSchema, SchemaColumn, generate_from_sql};

static EXAMPLE_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct ExampleDir(PathBuf);

impl ExampleDir {
    fn new() -> Self {
        let id = EXAMPLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("sql-tdg-readme-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test example directory must be creatable");
        Self(path)
    }
}

impl Drop for ExampleDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn readme_raw_sql_cli_example() {
    let workspace = ExampleDir::new();
    let output_dir = workspace.0.join("sql-tdg-output");
    let result = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--dialect",
            "generic",
            "--sql",
            "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
            "--schema",
            "orders:amount=INTEGER",
            "--matching",
            "50",
            "--rejected",
            "10",
            "--seed",
            "42",
            "--format",
            "csv",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled CLI must start");

    assert!(
        result.status.success(),
        "README raw SQL example failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let csv = fs::read_to_string(output_dir.join("0001-orders.csv"))
        .expect("CLI must export the documented CSV file");
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("amount"));
    let values = lines
        .map(|line| line.parse::<i32>().expect("amount must be an integer"))
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 60);
    assert!(values[..50].iter().all(|value| (10..20).contains(value)));
    assert!(values[50..].iter().all(|value| !(10..20).contains(value)));
    assert!(output_dir.join("metadata.sqltdg").is_file());
}

#[test]
fn readme_rust_library_example() -> Result<(), Box<dyn std::error::Error>> {
    let schema = RelationSchema::new(
        "orders",
        vec![SchemaColumn::from_sql_type("amount", "INTEGER", "generic")?],
    )?;

    let generated = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
        "generic",
        &[schema],
        100,
        42,
    )?;

    let amounts = generated
        .table("orders")
        .ok_or_else(|| std::io::Error::other("orders source was not generated"))?
        .get_ints("amount")?
        .ok_or_else(|| std::io::Error::other("amount column was not generated"))?;

    assert!(amounts.iter().all(|value| (10..20).contains(value)));
    Ok(())
}

#[test]
fn usage_binary_collation_cli_example() {
    let workspace = ExampleDir::new();
    let output_dir = workspace.0.join("sql-tdg-output");
    let result = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--dialect",
            "generic",
            "--sql",
            "SELECT name FROM customers WHERE name = 'Alice'",
            "--schema",
            "customers:name=VARCHAR",
            "--assume-comparison",
            "binary_collation",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled CLI must start");

    assert!(
        result.status.success(),
        "comparison assumption example failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output_dir.join("0001-customers.parquet").is_file());
    assert!(output_dir.join("metadata.sqltdg").is_file());
}

#[test]
fn readme_cte_join_aggregation_and_window_example() {
    let readme = include_str!("../README.md");
    let sql = readme
        .split_once("```sql\n")
        .and_then(|(_, sample)| sample.split_once("\n```"))
        .map(|(sample, _)| sample)
        .expect("README must include the documented analytic SQL example");

    let workspace = ExampleDir::new();
    let output_dir = workspace.0.join("sql-tdg-output");
    let result = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .args([
            "generate",
            "--dialect",
            "generic",
            "--sql",
            sql,
            "--schema",
            "orders:customer_id=INTEGER",
            "--schema",
            "orders:amount=INTEGER",
            "--schema",
            "customers:id=INTEGER",
            "--schema",
            "customers:segment=VARCHAR",
            "--schema",
            "customers:active=BOOLEAN",
            "--matching",
            "12",
            "--rejected",
            "0",
            "--seed",
            "42",
            "--output",
        ])
        .arg(&output_dir)
        .output()
        .expect("compiled CLI must start");

    assert!(
        result.status.success(),
        "README analytic SQL example failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8(result.stdout).expect("CLI output must be UTF-8");
    assert!(stdout.contains("relation=orders "), "{stdout}");
    assert!(stdout.contains("relation=customers "), "{stdout}");
    assert!(output_dir.join("metadata.sqltdg").is_file());
}

#[test]
fn readme_dbt_workflow_documents_catalog_generation() {
    let readme = include_str!("../README.md");
    let expected = "dbt compile\ndbt docs generate\nsql-tdg generate --dbt-project .";
    assert!(readme.contains(expected), "dbt catalog generation must be documented");
    assert!(readme.contains("data_type"), "catalog-less source fallback must be documented");
}
