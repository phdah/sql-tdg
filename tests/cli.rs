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
        stderr.contains("declare source columns with data_type")
            && stderr.contains("dbt docs generate"),
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

const DBT_DECLARED_MANIFEST: &str = include_str!("fixtures/dbt_manifest_declared.json");

fn run_manifest_cli(manifest_path: &Path, output_dir: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(manifest_path)
        .args(["--target", "warehouse.analytics.final_orders"])
        .args(["--matching", "8", "--format", "csv", "--output"])
        .arg(output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute")
}

#[test]
fn compiled_cli_generates_typed_dbt_sources_without_a_catalog() {
    let workspace = TestDir::new("dbt-manifest-types");
    let manifest_path = workspace.path().join("manifest.json");
    fs::write(&manifest_path, DBT_DECLARED_MANIFEST).expect("manifest should be writable");

    let output_dir = workspace.path().join("generated");
    let output = run_manifest_cli(&manifest_path, &output_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let csv = fs::read_to_string(output_dir.join("0001-warehouse.raw.orders.csv"))
        .expect("physical source with declared schema should be generated");
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("amount,id"));
    let rows = lines.collect::<Vec<_>>();
    assert_eq!(rows.len(), 8);
    for row in rows {
        let amount = row
            .split(',')
            .next()
            .expect("amount column")
            .parse::<i64>()
            .expect("integer amount");
        assert!((10..=50).contains(&amount), "unexpected amount: {amount}");
    }

    let metadata = fs::read_to_string(output_dir.join("metadata.sqltdg"))
        .expect("metadata should be exported");
    let metadata =
        sql_tdg::TestCaseMetadata::deserialize(&metadata).expect("metadata should round-trip");
    assert!(
        metadata
            .protocol()
            .document()
            .contains(r#""source_kind":"dbt_manifest""#),
        "protocol snapshot must identify manifest schema evidence"
    );
}

/// dbt writes `attached_node: null` for tests declared on sources; a relationships test between
/// two sources must not prevent analysis of the project.
#[test]
fn compiled_cli_generates_dbt_sources_with_unattached_source_relationships_test() {
    let workspace = TestDir::new("dbt-source-relationships");
    let manifest_path = workspace.path().join("manifest.json");
    fs::write(
        &manifest_path,
        include_str!("fixtures/dbt_manifest_source_relationships.json"),
    )
    .expect("manifest should be writable");

    let output_dir = workspace.path().join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(&manifest_path)
        .args(["--target", "warehouse.analytics.big_items"])
        .args(["--matching", "4", "--format", "csv", "--output"])
        .arg(&output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let csv = fs::read_to_string(output_dir.join("0001-warehouse.raw.order_items.csv"))
        .expect("source read by the model should be generated");
    let mut lines = csv.lines();
    let header = lines.next().expect("CSV header");
    let quantity_index = header
        .split(',')
        .position(|column| column == "quantity")
        .expect("quantity column");
    let rows = lines.collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let quantity = row
            .split(',')
            .nth(quantity_index)
            .expect("quantity value")
            .parse::<i64>()
            .expect("integer quantity");
        assert!(quantity >= 5, "unexpected quantity: {quantity}");
    }
}

/// Source-level dbt tests constrain the generated parents as well as the selected child.
#[test]
fn compiled_cli_honors_source_unique_not_null_accepted_values_and_relationships() {
    let workspace = TestDir::new("dbt-source-constraints");
    let manifest_path = workspace.path().join("manifest.json");
    fs::write(
        &manifest_path,
        include_str!("fixtures/dbt_manifest_constraints.json"),
    )
    .expect("manifest should be writable");

    let output_dir = workspace.path().join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(&manifest_path)
        .args(["--target", "warehouse.analytics.big_items"])
        .args([
            "--matching",
            "4",
            "--seed",
            "43",
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

    let items = fs::read_to_string(output_dir.join("0001-warehouse.raw.order_items.csv"))
        .expect("generated child source");
    let orders = fs::read_to_string(output_dir.join("0002-warehouse.raw.orders.csv"))
        .expect("generated foreign-key parent source");
    let mut unique_items = std::collections::BTreeSet::new();
    let mut unique_orders = std::collections::BTreeSet::new();
    for order in orders.lines().skip(1) {
        assert!(!order.is_empty());
        assert!(unique_orders.insert(order.to_owned()));
    }
    assert_eq!(unique_orders.len(), 4);

    for line in items.lines().skip(1) {
        let parts = line.split(',').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3, "row: {line}");
        assert!(!parts[0].is_empty());
        assert!(unique_items.insert(parts[0].to_owned()));
        assert!(
            unique_orders.contains(parts[1]),
            "orphan order_id: {}",
            parts[1]
        );
        assert!(
            ["5", "6", "7"].contains(&parts[2]),
            "invalid quantity: {}",
            parts[2]
        );
    }
    assert_eq!(unique_items.len(), 4);
}

#[test]
fn compiled_cli_rejects_unattributed_dbt_test_diagnostics() {
    let workspace = TestDir::new("dbt-unattributed-constraint");
    let manifest_path = workspace.path().join("manifest.json");
    let original = include_str!("fixtures/dbt_manifest_source_relationships.json");
    let invalid = original
        .replace(r#""name": "relationships""#, r#""name": "unique""#)
        .replace(
            r#""model": "{{ get_where_subquery(source('raw', 'order_items')) }}""#,
            r#""model": "{{ unsupported() }}""#,
        );
    assert_ne!(invalid, original);
    fs::write(&manifest_path, invalid).expect("manifest should be writable");
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(&manifest_path)
        .args(["--target", "warehouse.analytics.big_items"])
        .args(["--matching", "4", "--format", "csv", "--output"])
        .arg(workspace.path().join("generated"))
        .output()
        .expect("compiled sql-tdg binary should execute");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unattributed_dbt_test"),
        "unexpected constraint diagnostic: {stderr}"
    );
}

#[test]
fn compiled_cli_accepts_dbt_project_without_a_catalog() {
    let workspace = TestDir::new("dbt-project-manifest-types");
    let target_dir = workspace.path().join("target");
    fs::create_dir_all(&target_dir).expect("target directory should be creatable");
    fs::write(target_dir.join("manifest.json"), DBT_DECLARED_MANIFEST)
        .expect("manifest should be writable");

    let output_dir = workspace.path().join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-project")
        .arg(workspace.path())
        .args(["--target", "warehouse.analytics.final_orders"])
        .args(["--matching", "2", "--format", "csv", "--output"])
        .arg(&output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_dir.join("0001-warehouse.raw.orders.csv").is_file());
}

#[test]
fn compiled_cli_rejects_missing_declared_dbt_column_type() {
    let workspace = TestDir::new("dbt-missing-declared-type");
    let manifest_path = workspace.path().join("manifest.json");
    let without_type = DBT_DECLARED_MANIFEST.replace(
        r#""name": "amount", "data_type": "BIGINT""#,
        r#""name": "amount""#,
    );
    assert_ne!(without_type, DBT_DECLARED_MANIFEST);
    fs::write(&manifest_path, without_type).expect("manifest should be writable");

    let output = run_manifest_cli(&manifest_path, &workspace.path().join("generated"));
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        stderr.contains("warehouse.raw.orders")
            && stderr.contains("amount")
            && stderr.contains("missing declared data_type")
            && stderr.contains("add data_type"),
        "stderr: {stderr}"
    );
}

#[test]
fn compiled_cli_rejects_undeclared_dbt_source_column() {
    let workspace = TestDir::new("dbt-undeclared-column");
    let manifest_path = workspace.path().join("manifest.json");
    let unknown_column = DBT_DECLARED_MANIFEST.replace("amount >= 10", "ghost >= 10");
    assert_ne!(unknown_column, DBT_DECLARED_MANIFEST);
    fs::write(&manifest_path, unknown_column).expect("manifest should be writable");

    let output = run_manifest_cli(&manifest_path, &workspace.path().join("generated"));
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        stderr.contains("unknown_schema_column") && stderr.contains("ghost"),
        "stderr: {stderr}"
    );
}

#[test]
fn compiled_cli_requires_an_explicitly_requested_dbt_catalog() {
    let workspace = TestDir::new("dbt-explicit-missing-catalog");
    let manifest_path = workspace.path().join("manifest.json");
    fs::write(&manifest_path, DBT_DECLARED_MANIFEST).expect("manifest should be writable");
    let catalog_path = workspace.path().join("not-there.json");

    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(&manifest_path)
        .arg("--dbt-catalog")
        .arg(&catalog_path)
        .arg("--output")
        .arg(workspace.path().join("generated"))
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("not-there.json"),
        "explicitly requested catalog must not be silently ignored"
    );
}
