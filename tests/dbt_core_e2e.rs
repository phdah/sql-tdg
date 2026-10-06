use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "support/dbt_duckdb.rs"]
mod dbt_duckdb;

use dbt_duckdb::DbtDuckDb;

static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

const TARGET_RELATION: &str = r#""fixture"."raw_analytics"."final_orders""#;
const STG_ORDERS_RELATION: &str = r#""fixture"."raw_analytics"."stg_orders""#;
const STG_CUSTOMERS_RELATION: &str = r#""fixture"."raw_analytics"."stg_customers""#;

struct DbtProject {
    path: PathBuf,
    database: PathBuf,
}

impl DbtProject {
    fn new() -> Self {
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sql-tdg-dbt-e2e-{}-{counter}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        copy_dir_all(&fixture_root(), &path).expect("dbt fixture should copy");
        let database = path.join("fixture.duckdb");
        Self { path, database }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn database(&self) -> &Path {
        &self.database
    }
}

impl Drop for DbtProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dbt_core_project")
}

fn copy_dir_all(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn run_dbt(project: &DbtProject, args: &[&str]) -> Output {
    Command::new("dbt")
        .args(args)
        .arg("--project-dir")
        .arg(project.path())
        .arg("--profiles-dir")
        .arg(project.path())
        .env("DBT_E2E_DATABASE", project.database())
        .current_dir(project.path())
        .output()
        .expect("dbt should be installed for the dedicated dbt-e2e target")
}

fn assert_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bootstrap_artifacts(project: &DbtProject) {
    assert_success("dbt seed", &run_dbt(project, &["seed", "--full-refresh"]));
    assert_success("dbt run", &run_dbt(project, &["run", "--full-refresh"]));
    assert_success(
        "dbt docs generate",
        &run_dbt(project, &["docs", "generate"]),
    );

    assert!(project.path().join("target/manifest.json").is_file());
    assert!(project.path().join("target/catalog.json").is_file());
}

fn run_tdg(project: &DbtProject, output_dir: &Path, boundaries: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sql-tdg"));
    command
        .arg("generate")
        .arg("--dbt-project")
        .arg(project.path())
        .args(["--target", TARGET_RELATION])
        .args(["--matching", "1", "--rejected", "1"])
        .args(["--seed", "42", "--format", "csv"])
        .arg("--output")
        .arg(output_dir);

    for boundary in boundaries {
        command.args(["--boundary", boundary]);
    }

    command
        .output()
        .expect("compiled sql-tdg binary should execute")
}

fn generated_paths(stdout: &[u8]) -> BTreeMap<String, PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("relation=")?;
            let (relation, path) = line.split_once(" path=")?;
            Some((relation.to_owned(), PathBuf::from(path)))
        })
        .collect()
}

fn relation_leaf(relation: &str) -> String {
    relation
        .rsplit('.')
        .next()
        .unwrap_or(relation)
        .trim()
        .trim_matches('"')
        .to_owned()
}

fn assert_generated_rows(paths: &BTreeMap<String, PathBuf>, expected_relations: &[&str]) {
    let actual = paths
        .keys()
        .map(|relation| relation_leaf(relation))
        .collect::<BTreeSet<_>>();
    let expected = expected_relations
        .iter()
        .map(|relation| (*relation).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);

    for path in paths.values() {
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert_eq!(
            contents.lines().skip(1).count(),
            2,
            "expected one matching and one rejected row in {}",
            path.display()
        );
    }
}

fn install_source_seeds(project: &DbtProject, paths: &BTreeMap<String, PathBuf>) {
    for (relation, path) in paths {
        let table = relation_leaf(relation);
        let destination = project.path().join("seeds").join(format!("{table}.csv"));
        fs::copy(path, &destination).unwrap_or_else(|error| {
            panic!(
                "failed to replace dbt seed {} from {}: {error}",
                destination.display(),
                path.display()
            )
        });
    }
}

fn row(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn physical_snapshot(project: &DbtProject) -> Vec<Vec<String>> {
    let database = DbtDuckDb::open(project.database()).expect("DuckDB fixture should open");
    database
        .execute_text(
            r#"
SELECT 'aggregate_summary', region::VARCHAR, order_count::VARCHAR, max_amount::VARCHAR
FROM raw_analytics.aggregate_summary
UNION ALL
SELECT 'derived_orders', amount_bucket::VARCHAR, paid_flag::VARCHAR, ''
FROM raw_analytics.derived_orders
UNION ALL
SELECT 'final_orders', amount::VARCHAR, status::VARCHAR, region::VARCHAR
FROM raw_analytics.final_orders
UNION ALL
SELECT 'independent_return_summary', reason::VARCHAR, return_count::VARCHAR, max_refund::VARCHAR
FROM raw_analytics.independent_return_summary
UNION ALL
SELECT 'ranked_orders', region::VARCHAR, amount::VARCHAR, rn::VARCHAR
FROM raw_analytics.ranked_orders
UNION ALL
SELECT 'subquery_orders', region::VARCHAR, amount::VARCHAR, ''
FROM raw_analytics.subquery_orders
UNION ALL
SELECT 'unioned_orders', region::VARCHAR, amount::VARCHAR, ''
FROM raw_analytics.unioned_orders
ORDER BY 1, 2, 3, 4
"#,
            4,
        )
        .expect("dbt result snapshot should be queryable")
}

fn expected_physical_snapshot() -> Vec<Vec<String>> {
    vec![
        row(&["aggregate_summary", "north", "2", "42"]),
        row(&["derived_orders", "medium", "1", ""]),
        row(&["derived_orders", "medium", "1", ""]),
        row(&["final_orders", "42", "paid", "north"]),
        row(&["final_orders", "42", "paid", "north"]),
        row(&["independent_return_summary", "full", "1", "70"]),
        row(&["ranked_orders", "north", "42", "1"]),
        row(&["ranked_orders", "north", "42", "2"]),
        row(&["subquery_orders", "north", "42", ""]),
        row(&["subquery_orders", "north", "42", ""]),
        row(&["unioned_orders", "north", "42", ""]),
        row(&["unioned_orders", "north", "42", ""]),
        row(&["unioned_orders", "north", "42", ""]),
    ]
}

fn final_snapshot(project: &DbtProject) -> Vec<Vec<String>> {
    let database = DbtDuckDb::open(project.database()).expect("DuckDB fixture should open");
    database
        .execute_text(
            "SELECT amount::VARCHAR, status::VARCHAR, region::VARCHAR \
             FROM raw_analytics.final_orders ORDER BY amount, status, region",
            3,
        )
        .expect("final model should be queryable")
}

fn reset_database(project: &DbtProject) {
    if project.database().exists() {
        fs::remove_file(project.database()).expect("fixture database should be removable");
    }
    let wal = PathBuf::from(format!("{}.wal", project.database().display()));
    if wal.exists() {
        fs::remove_file(wal).expect("fixture WAL should be removable");
    }
}

fn materialize_boundaries(project: &DbtProject, paths: &BTreeMap<String, PathBuf>) {
    reset_database(project);
    let database = DbtDuckDb::open(project.database()).expect("DuckDB fixture should open");
    for (relation, path) in paths {
        database
            .materialize_csv(relation, path)
            .unwrap_or_else(|error| panic!("failed to materialize {relation}: {error}"));
    }
}

fn mutate_final_predicate(project: &DbtProject) {
    let path = project.path().join("models/final_orders.sql");
    let original = fs::read_to_string(&path).expect("final model should be readable");
    let mutated = original.replace("where amount = 42", "where amount = 43");
    assert_ne!(mutated, original, "fixture predicate should be replaceable");
    fs::write(path, mutated).expect("mutated final model should be writable");
}

#[test]
#[ignore = "requires dbt Core and dbt-duckdb; run make dbt-e2e"]
fn dbt_core_duckdb_workflow_generates_sources_and_intermediate_boundaries() {
    let project = DbtProject::new();
    bootstrap_artifacts(&project);

    let physical_output_dir = project.path().join("generated/physical");
    let physical = run_tdg(&project, &physical_output_dir, &[]);
    assert_success("sql-tdg physical source generation", &physical);
    let physical_paths = generated_paths(&physical.stdout);
    assert_generated_rows(&physical_paths, &["customers", "orders"]);
    install_source_seeds(&project, &physical_paths);

    assert_success(
        "dbt seed generated sources",
        &run_dbt(&project, &["seed", "--full-refresh"]),
    );
    assert_success(
        "dbt run generated sources",
        &run_dbt(&project, &["run", "--full-refresh"]),
    );
    assert_eq!(physical_snapshot(&project), expected_physical_snapshot());

    let boundary_output_dir = project.path().join("generated/boundary");
    let boundary = run_tdg(
        &project,
        &boundary_output_dir,
        &[STG_ORDERS_RELATION, STG_CUSTOMERS_RELATION],
    );
    assert_success("sql-tdg intermediate boundary generation", &boundary);
    let boundary_paths = generated_paths(&boundary.stdout);
    assert_generated_rows(&boundary_paths, &["stg_customers", "stg_orders"]);
    materialize_boundaries(&project, &boundary_paths);

    assert_success(
        "dbt downstream boundary run",
        &run_dbt(
            &project,
            &["run", "--select", "enriched_orders", "final_orders"],
        ),
    );
    let expected_final = vec![row(&["42", "paid", "north"])];
    assert_eq!(final_snapshot(&project), expected_final);

    mutate_final_predicate(&project);
    assert_success(
        "dbt mutated final run",
        &run_dbt(&project, &["run", "--select", "final_orders"]),
    );
    let mutated = final_snapshot(&project);
    assert_ne!(mutated, expected_final);
    assert!(
        mutated.is_empty(),
        "mutated predicate should reject the witness"
    );
}
