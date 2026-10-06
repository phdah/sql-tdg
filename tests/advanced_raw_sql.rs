use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "support/cli_duckdb.rs"]
mod cli_duckdb;

use cli_duckdb::CliDuckDb;

static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

const PIPELINE_SCHEMAS: &[&str] = &[
    "raw_orders:order_id=INTEGER",
    "raw_orders:customer_id=INTEGER",
    "raw_orders:amount=INTEGER",
    "raw_customers:customer_id=INTEGER",
    "raw_customers:active=BOOLEAN",
    "stage_orders:order_id=INTEGER",
    "stage_orders:customer_id=INTEGER",
    "stage_orders:amount=INTEGER",
    "stage_orders:amount_bucket=VARCHAR",
    "stage_customers:customer_id=INTEGER",
    "stage_customers:active=BOOLEAN",
    "core_enriched:customer_id=INTEGER",
    "core_enriched:amount=INTEGER",
    "core_enriched:amount_bucket=VARCHAR",
];

const CORE_FROM_STAGE: &str = r#"
CREATE VIEW core_enriched AS
SELECT
    orders.customer_id,
    orders.amount,
    orders.amount_bucket
FROM stage_orders AS orders
JOIN stage_customers AS customers
  ON orders.customer_id = customers.customer_id;

SELECT amount_bucket::VARCHAR, amount::VARCHAR
FROM core_enriched
ORDER BY amount_bucket, amount;
"#;

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sql-tdg-advanced-{name}-{}-{counter}",
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

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("raw_sql")
        .join(name)
}

fn run_cli(
    fixture_path: &Path,
    target: &str,
    schemas: &[&str],
    boundaries: &[&str],
    output_dir: &Path,
) -> Output {
    run_cli_with_counts(fixture_path, target, schemas, boundaries, output_dir, 1, 1)
}

fn run_cli_with_counts(
    fixture_path: &Path,
    target: &str,
    schemas: &[&str],
    boundaries: &[&str],
    output_dir: &Path,
    matching: usize,
    rejected: usize,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sql-tdg"));
    command
        .args(["generate", "--dialect", "duckdb", "--file"])
        .arg(fixture_path);

    for schema in schemas {
        command.args(["--schema", schema]);
    }
    command.args(["--target", target]);
    for boundary in boundaries {
        command.args(["--boundary", boundary]);
    }
    command
        .arg("--matching")
        .arg(matching.to_string())
        .arg("--rejected")
        .arg(rejected.to_string())
        .args(["--seed", "42", "--format", "parquet", "--output"])
        .arg(output_dir)
        .output()
        .expect("compiled sql-tdg binary should execute")
}

fn assert_cli_success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("CLI stdout should be UTF-8")
}

fn generated_paths(stdout: &str) -> BTreeMap<String, PathBuf> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("relation=")?;
            let (relation, path) = line.split_once(" path=")?;
            Some((relation.to_owned(), PathBuf::from(path)))
        })
        .collect()
}

fn load_outputs(executor: &CliDuckDb, stdout: &str) {
    let outputs = generated_paths(stdout);
    assert!(!outputs.is_empty(), "CLI should report generated relations");
    for (relation, path) in outputs {
        assert!(
            path.is_file(),
            "generated output should exist: {}",
            path.display()
        );
        executor
            .materialize_parquet(&relation, &path)
            .expect("CLI Parquet output should materialize in DuckDB");
    }
}

fn execute_with_final_select(
    executor: &CliDuckDb,
    fixture_sql: &str,
    final_select: &str,
    column_count: usize,
) -> Vec<Vec<String>> {
    executor
        .execute_text(&format!("{fixture_sql}\n{final_select}"), column_count)
        .expect("fixture workload should execute")
}

fn expected_row(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn cli_pipeline_executes_from_physical_sources_and_intermediate_boundary() {
    let fixture_path = fixture("advanced_pipeline.sql");
    let fixture_sql =
        fs::read_to_string(&fixture_path).expect("pipeline fixture should be readable");

    let physical_dir = TestDir::new("pipeline-physical");
    let physical = run_cli(
        &fixture_path,
        "mart_customer_summary",
        PIPELINE_SCHEMAS,
        &[],
        physical_dir.path(),
    );
    let physical_stdout = assert_cli_success(&physical);
    let physical_db = CliDuckDb::in_memory().expect("DuckDB should open");
    load_outputs(&physical_db, &physical_stdout);
    let physical_result = execute_with_final_select(
        &physical_db,
        &fixture_sql,
        "SELECT amount_bucket::VARCHAR, order_count::VARCHAR, max_amount::VARCHAR FROM mart_customer_summary ORDER BY amount_bucket",
        3,
    );

    assert_eq!(physical_result, vec![expected_row(&["high", "2", "100"])]);

    let mutation = physical_db
        .execute_text(
            "SELECT orders.amount_bucket::VARCHAR, COUNT(*)::VARCHAR AS order_count, \
                    MAX(orders.amount)::VARCHAR AS max_amount \
             FROM stage_orders AS orders \
             CROSS JOIN stage_customers AS customers \
             WHERE orders.amount = 100 \
             GROUP BY orders.amount_bucket \
             HAVING COUNT(*) >= 1 \
             ORDER BY orders.amount_bucket",
            3,
        )
        .expect("mutated workload should execute");
    assert_ne!(
        mutation, physical_result,
        "removing the relational stage must change the complete result"
    );
    assert_eq!(mutation, vec![expected_row(&["high", "4", "100"])]);

    let boundary_fixture_path = fixture("advanced_boundary.sql");
    let boundary_dir = TestDir::new("pipeline-boundary");
    let boundary = run_cli(
        &boundary_fixture_path,
        "core_enriched",
        PIPELINE_SCHEMAS,
        &["stage_orders", "stage_customers"],
        boundary_dir.path(),
    );
    let boundary_stdout = assert_cli_success(&boundary);
    let boundary_paths = generated_paths(&boundary_stdout);
    assert_eq!(
        boundary_paths.keys().cloned().collect::<Vec<_>>(),
        vec!["stage_customers".to_owned(), "stage_orders".to_owned()]
    );

    let boundary_db = CliDuckDb::in_memory().expect("DuckDB should open");
    load_outputs(&boundary_db, &boundary_stdout);
    let boundary_result = boundary_db
        .execute_text(CORE_FROM_STAGE, 2)
        .expect("intermediate-boundary workload should execute");

    assert_eq!(
        boundary_result,
        vec![
            expected_row(&["high", "100"]),
            expected_row(&["high", "100"]),
        ]
    );
}

#[test]
fn cli_samples_full_matching_and_rejected_integer_ranges() {
    const MATCHING: usize = 64;
    const REJECTED: usize = 64;

    let fixture_path = fixture("range_sampling.sql");
    let output_dir = TestDir::new("range-sampling");
    let output = run_cli_with_counts(
        &fixture_path,
        "range_result",
        &["raw_range:value=INTEGER"],
        &[],
        output_dir.path(),
        MATCHING,
        REJECTED,
    );
    let stdout = assert_cli_success(&output);
    let executor = CliDuckDb::in_memory().expect("DuckDB should open");
    load_outputs(&executor, &stdout);

    let rows = executor
        .execute_text("SELECT value::VARCHAR FROM raw_range", 1)
        .expect("generated range source should be queryable");
    assert_eq!(rows.len(), MATCHING + REJECTED);

    let values = rows
        .iter()
        .map(|row| {
            row[0]
                .parse::<i32>()
                .expect("generated INTEGER should parse as i32")
        })
        .collect::<Vec<_>>();
    let matching = values
        .iter()
        .copied()
        .filter(|value| (10..=1_000).contains(value))
        .collect::<Vec<_>>();
    let rejected = values
        .iter()
        .copied()
        .filter(|value| !(10..=1_000).contains(value))
        .collect::<Vec<_>>();

    assert_eq!(matching.len(), MATCHING);
    assert_eq!(rejected.len(), REJECTED);
    assert!(
        matching.iter().copied().collect::<BTreeSet<_>>().len() > 1,
        "matching rows must be sampled across the allowed interval"
    );
    assert!(
        rejected.iter().copied().collect::<BTreeSet<_>>().len() > 1,
        "rejected rows must be sampled across the complement interval"
    );
}

#[test]
fn cli_window_fixture_is_ranked_limited_and_deterministic() {
    let fixture_path = fixture("window_ranked.sql");
    let fixture_sql = fs::read_to_string(&fixture_path).expect("window fixture should be readable");
    let schemas = &["raw_window:category=VARCHAR", "raw_window:score=INTEGER"];

    let first_dir = TestDir::new("window-first");
    let first = run_cli(
        &fixture_path,
        "ranked_result",
        schemas,
        &[],
        first_dir.path(),
    );
    let first_stdout = assert_cli_success(&first);
    let first_paths = generated_paths(&first_stdout);

    let second_dir = TestDir::new("window-second");
    let second = run_cli(
        &fixture_path,
        "ranked_result",
        schemas,
        &[],
        second_dir.path(),
    );
    let second_stdout = assert_cli_success(&second);
    let second_paths = generated_paths(&second_stdout);

    assert_eq!(
        first_paths.keys().collect::<Vec<_>>(),
        second_paths.keys().collect::<Vec<_>>()
    );
    for relation in first_paths.keys() {
        assert_eq!(
            fs::read(&first_paths[relation]).expect("first Parquet file should be readable"),
            fs::read(&second_paths[relation]).expect("second Parquet file should be readable"),
            "fixed seed should reproduce identical Parquet output for {relation}"
        );
    }

    let executor = CliDuckDb::in_memory().expect("DuckDB should open");
    load_outputs(&executor, &first_stdout);
    let result = execute_with_final_select(
        &executor,
        &fixture_sql,
        "SELECT marker::VARCHAR, rn::VARCHAR FROM ranked_result ORDER BY rn",
        2,
    );
    assert_eq!(result, vec![expected_row(&["1", "1"])]);
}

#[test]
fn cli_set_operation_fixtures_execute_with_full_expected_results() {
    let fixture_path = fixture("set_operations.sql");
    let schemas = &[
        "raw_a:value=INTEGER",
        "raw_b:value=INTEGER",
        "raw_c:value=INTEGER",
    ];
    let cases = [
        (
            "union_all_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW set_b AS SELECT 2 AS marker FROM raw_b WHERE value = 20; \
             CREATE VIEW union_all_result AS \
             SELECT marker FROM set_a UNION ALL SELECT marker FROM set_b ORDER BY marker LIMIT 2; \
             SELECT marker::VARCHAR FROM union_all_result ORDER BY marker;",
            vec![expected_row(&["1"]), expected_row(&["2"])],
        ),
        (
            "union_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW set_c AS SELECT 1 AS marker FROM raw_c WHERE value = 30; \
             CREATE VIEW union_result AS \
             SELECT marker FROM set_a UNION SELECT marker FROM set_c ORDER BY marker; \
             SELECT marker::VARCHAR FROM union_result ORDER BY marker;",
            vec![expected_row(&["1"])],
        ),
        (
            "intersect_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW set_c AS SELECT 1 AS marker FROM raw_c WHERE value = 30; \
             CREATE VIEW intersect_result AS \
             SELECT marker FROM set_a INTERSECT SELECT marker FROM set_c ORDER BY marker; \
             SELECT marker::VARCHAR FROM intersect_result ORDER BY marker;",
            vec![expected_row(&["1"])],
        ),
        (
            "except_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW set_b AS SELECT 2 AS marker FROM raw_b WHERE value = 20; \
             CREATE VIEW except_result AS \
             SELECT marker FROM set_a EXCEPT SELECT marker FROM set_b ORDER BY marker; \
             SELECT marker::VARCHAR FROM except_result ORDER BY marker;",
            vec![expected_row(&["1"])],
        ),
        (
            "distinct_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW distinct_result AS SELECT DISTINCT marker FROM set_a ORDER BY marker; \
             SELECT marker::VARCHAR FROM distinct_result ORDER BY marker;",
            vec![expected_row(&["1"])],
        ),
        (
            "limited_result",
            "CREATE VIEW set_a AS SELECT 1 AS marker FROM raw_a WHERE value = 10; \
             CREATE VIEW set_b AS SELECT 2 AS marker FROM raw_b WHERE value = 20; \
             CREATE VIEW limited_result AS \
             SELECT marker FROM set_a UNION ALL SELECT marker FROM set_b ORDER BY marker LIMIT 1; \
             SELECT marker::VARCHAR FROM limited_result ORDER BY marker;",
            vec![expected_row(&["1"])],
        ),
    ];

    for (index, (target, workload, expected)) in cases.into_iter().enumerate() {
        let output_dir = TestDir::new(&format!("set-{index}"));
        let output = run_cli(&fixture_path, target, schemas, &[], output_dir.path());
        let stdout = assert_cli_success(&output);
        let executor = CliDuckDb::in_memory().expect("DuckDB should open");
        load_outputs(&executor, &stdout);
        let result = executor
            .execute_text(workload, 1)
            .unwrap_or_else(|error| panic!("{target} should execute: {error}"));
        assert_eq!(result, expected, "target {target}");
    }
}

#[test]
fn cli_insert_fixture_executes_deterministic_dml() {
    let fixture_path = fixture("insert_workflow.sql");
    let fixture_sql = fs::read_to_string(&fixture_path).expect("INSERT fixture should be readable");
    let output_dir = TestDir::new("insert");
    let output = run_cli(
        &fixture_path,
        "insert_sink",
        &["raw_insert:value=INTEGER"],
        &[],
        output_dir.path(),
    );
    let stdout = assert_cli_success(&output);
    let executor = CliDuckDb::in_memory().expect("DuckDB should open");
    load_outputs(&executor, &stdout);
    let result = executor
        .execute_text(
            &format!(
                "CREATE TABLE insert_sink(value INTEGER);\n{fixture_sql}\n\
                 SELECT value::VARCHAR FROM insert_sink ORDER BY value"
            ),
            1,
        )
        .expect("INSERT workload should execute");
    assert_eq!(result, vec![expected_row(&["10"])]);
}

#[test]
fn cli_reports_exists_relationship_as_explicitly_unsupported() {
    let fixture_path = fixture("unsupported_exists.sql");
    let output_dir = TestDir::new("exists");
    let output = run_cli(
        &fixture_path,
        "exists_result",
        &[
            "raw_exists_orders:customer_id=INTEGER",
            "raw_exists_orders:amount=INTEGER",
            "raw_exists_customers:customer_id=INTEGER",
        ],
        &[],
        output_dir.path(),
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr should be UTF-8");
    assert!(
        stderr.contains(
            "EXISTS/NOT EXISTS relationship semantics are not yet supported by relational witness generation"
        ),
        "unexpected stderr: {stderr}"
    );
}
