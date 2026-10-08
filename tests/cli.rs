use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn help_is_readable_and_plain_when_redirected() {
    for args in [
        vec![],
        vec!["--help"],
        vec!["-h"],
        vec!["help"],
        vec!["generate", "--help"],
        vec!["generate", "-h"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
            .args(&args)
            .env("TERM", "xterm-256color")
            .output()
            .expect("CLI help should run");

        assert!(output.status.success(), "help failed for {args:?}");
        assert!(output.stderr.is_empty(), "unexpected stderr for {args:?}");
        let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");
        assert!(
            !stdout.contains("\x1b["),
            "redirected help must not have ANSI"
        );
        assert!(stdout.contains("RAW SQL INPUT\n"));
        assert!(stdout.contains("DBT INPUT\n"));
        assert!(stdout.contains("GENERATION OPTIONS\n"));
        assert!(stdout.contains("OUTPUT OPTIONS\n"));
        assert!(stdout.contains("EXAMPLES\n"));
        assert!(stdout.contains("--assume-comparison"));
        assert!(stdout.lines().all(|line| line.len() <= 80));
    }
}

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

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.contains("rejected_rows=10"),
        "rejected rows default to 10: {stdout}"
    );
    assert_eq!(csv.lines().skip(1).count(), 12);
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
            "--rejected",
            "0",
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
        .args([
            "--matching",
            "8",
            "--rejected",
            "0",
            "--format",
            "csv",
            "--output",
        ])
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
        .args([
            "--matching",
            "4",
            "--rejected",
            "0",
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
            "--rejected",
            "0",
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
    let mut referenced_orders = std::collections::BTreeSet::new();
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
        referenced_orders.insert(parts[1].to_owned());
        assert!(
            ["5", "6", "7"].contains(&parts[2]),
            "invalid quantity: {}",
            parts[2]
        );
    }
    assert_eq!(unique_items.len(), 4);
    assert!(
        referenced_orders.len() >= 3,
        "child rows must reference multiple parent keys: {referenced_orders:?}"
    );
    assert!(
        unique_orders.iter().all(|key| {
            key.parse::<i64>()
                .is_ok_and(|value| (1..=1000).contains(&value))
        }),
        "parent keys should be moderate: {unique_orders:?}"
    );
}

#[test]
fn compiled_cli_generates_relationship_parent_without_a_model_dependency() {
    let workspace = TestDir::new("dbt-relationship-only-parent");
    let manifest_path = workspace.path().join("manifest.json");
    // Only order_items is consumed by model SQL; orders is declared as a source
    // and referenced exclusively by the dbt relationships test.
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
        .args([
            "--matching",
            "4",
            "--rejected",
            "0",
            "--seed",
            "42",
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
    let parent = fs::read_to_string(output_dir.join("0002-warehouse.raw.orders.csv"))
        .expect("foreign-key-only parent source should be generated");
    let parent_keys = parent
        .lines()
        .skip(1)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(parent.lines().skip(1).count(), 4);
    let child = fs::read_to_string(output_dir.join("0001-warehouse.raw.order_items.csv"))
        .expect("child source should be generated");
    let mut child_count = 0;
    for row in child.lines().skip(1) {
        let columns = row.split(',').collect::<Vec<_>>();
        assert_eq!(columns.len(), 3);
        assert!(
            parent_keys.contains(columns[1]),
            "generated child has orphaned order_id: {}",
            columns[1]
        );
        child_count += 1;
    }
    assert_eq!(child_count, 4);
}

#[test]
fn compiled_cli_rejects_accepted_values_conflicting_with_query_domain() {
    let workspace = TestDir::new("dbt-accepted-value-conflict");
    let manifest_path = workspace.path().join("manifest.json");
    let original = include_str!("fixtures/dbt_manifest_constraints.json");
    let conflicting = original.replace("quantity >= 5", "quantity >= 500");
    assert_ne!(conflicting, original);
    fs::write(&manifest_path, conflicting).expect("manifest should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("generate")
        .arg("--dbt-manifest")
        .arg(&manifest_path)
        .args(["--target", "warehouse.analytics.big_items"])
        .args([
            "--matching",
            "4",
            "--rejected",
            "0",
            "--format",
            "csv",
            "--output",
        ])
        .arg(workspace.path().join("generated"))
        .output()
        .expect("compiled sql-tdg binary should execute");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("relation constraint") && stderr.contains("quantity"),
        "unexpected conflicting-domain diagnostic: {stderr}"
    );
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

#[test]
fn scenario_cli_partitions_compiled_dbt_outcomes_with_separate_metadata() {
    let workspace = TestDir::new("dbt-scenarios");
    let manifest = workspace.path().join("manifest.json");
    fs::write(
        &manifest,
        include_str!("fixtures/dbt_manifest_scenarios.json"),
    )
    .expect("dbt manifest fixture should be writable");

    let output_dir = workspace.path().join("scenarios");
    let run = |directory: &Path| {
        Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
            .args(["generate", "--dbt-manifest"])
            .arg(&manifest)
            .args([
                "--scenarios",
                "--matching",
                "8",
                "--rejected",
                "0",
                "--format",
                "csv",
                "--seed",
                "42",
                "--output",
            ])
            .arg(directory)
            .output()
            .expect("CLI should execute")
    };

    let output = run(&output_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("scenario=scenario-0001 outcomes="));
    assert!(stdout.contains("scenario=scenario-0002 outcomes="));
    assert!(!output_dir.join("scenario-0003").exists());

    let first = output_dir.join("scenario-0001");
    let second = output_dir.join("scenario-0002");
    let first_metadata = sql_tdg::TestCaseMetadata::deserialize(
        &fs::read_to_string(first.join("metadata.sqltdg")).expect("first metadata"),
    )
    .expect("first metadata should round trip");
    let second_metadata = sql_tdg::TestCaseMetadata::deserialize(
        &fs::read_to_string(second.join("metadata.sqltdg")).expect("second metadata"),
    )
    .expect("second metadata should round trip");
    assert_eq!(
        first_metadata.target().kind(),
        sql_tdg::TargetKind::ScenarioOutcomes
    );
    assert_eq!(
        second_metadata.target().kind(),
        sql_tdg::TargetKind::ScenarioOutcomes
    );
    assert!(
        first_metadata
            .target()
            .identifier()
            .contains("active_orders")
    );
    assert!(
        first_metadata
            .target()
            .identifier()
            .contains("final_orders")
    );
    assert!(!first_metadata.target().identifier().contains("rich_orders"));
    assert!(
        second_metadata
            .target()
            .identifier()
            .contains("rich_orders")
    );

    let source_name = "0001-warehouse.raw.orders.csv";
    let first_csv = fs::read_to_string(first.join(source_name)).expect("first generated dataset");
    let second_csv =
        fs::read_to_string(second.join(source_name)).expect("second generated dataset");
    assert_eq!(first_csv.lines().count(), 9);
    assert_eq!(second_csv.lines().count(), 9);

    let repeat_dir = workspace.path().join("repeat");
    let again = run(&repeat_dir);
    assert!(
        again.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        first_csv,
        fs::read_to_string(repeat_dir.join("scenario-0001").join(source_name))
            .expect("same first dataset")
    );
    assert_eq!(
        second_csv,
        fs::read_to_string(repeat_dir.join("scenario-0002").join(source_name))
            .expect("same second dataset")
    );

    // Materialize and execute the compiled dbt SQL independently against each scenario.
    for (index, target) in [(1, "final_orders"), (2, "rich_orders")] {
        let csv = output_dir
            .join(format!("scenario-{index:04}"))
            .join(source_name);
        let db = duckdb::Connection::open_in_memory().expect("DuckDB connection");
        db.execute_batch("ATTACH ':memory:' AS warehouse; CREATE SCHEMA warehouse.raw; CREATE SCHEMA warehouse.analytics;")
            .expect("catalog schemas");
        let escaped_path = csv.to_string_lossy().replace('\'', "''");
        db.execute_batch(&format!(
            "CREATE TABLE warehouse.raw.orders AS SELECT * FROM read_csv_auto('{escaped_path}')"
        ))
        .expect("load scenario source");
        db.execute_batch(
            "CREATE VIEW warehouse.analytics.stg_orders AS
             SELECT id, amount FROM warehouse.raw.orders WHERE amount >= 10 AND amount <= 100;",
        )
        .expect("execute compiled staging model");
        let predicate = match target {
            "final_orders" => "amount <= 50",
            "rich_orders" => "amount >= 80",
            _ => unreachable!("only two targets"),
        };
        let sql = format!("SELECT COUNT(*) FROM warehouse.analytics.stg_orders WHERE {predicate}");
        let matches: i64 = db
            .query_row(&sql, [], |row| row.get(0))
            .expect("execute terminal dbt predicate");
        assert_eq!(
            matches, 8,
            "every scenario source row should satisfy its dbt model"
        );
    }
}

#[test]
fn scenario_cli_requires_explicit_matching_only_and_rejects_selectors() {
    let workspace = TestDir::new("scenario-validation");
    for extra in [vec![], vec!["--rejected", "0", "--target", "orders"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
            .args([
                "generate",
                "--sql",
                "SELECT amount FROM orders",
                "--schema",
                "orders:amount=INTEGER",
                "--scenarios",
            ])
            .args(&extra)
            .arg("--output")
            .arg(workspace.path().join("output"))
            .output()
            .expect("CLI should execute");
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--scenarios"), "stderr: {stderr}");
    }
}
