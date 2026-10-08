use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sql_semantic_protocol::{
    AnalysisBundle, ComparisonAssumption, ConfiguredSqlInput, DatasetRef, DbtArtifactsError,
    RelationCatalog, RelationSchema, SchemaColumn, SqlInput,
    analyze_configured_inputs_with_catalog, analyze_dbt_artifacts,
    analyze_dbt_manifest_with_schemas, dialect_from_name, parse_dbt_catalog, parse_dbt_manifest,
    to_bundle_json,
};
use sql_tdg::{
    GeneratedData, GeneratedRelation, GenerationBoundary, GenerationRowCounts, OutcomeSelector,
    ProtocolSnapshot, TargetKind, TestCaseMetadata, TestTarget, WorkloadIdentity,
    generate_classified_from_bundle_at_boundary, write_csv, write_parquet,
};

const HELP: &str = r#"sql-tdg
Generate deterministic, backend-neutral SQL test data.

USAGE
  sql-tdg generate [OPTIONS]
  sql-tdg --help
  sql-tdg --version

RAW SQL INPUT
  --sql <SQL>                     Inline SQL (repeatable)
  --file <path>                   SQL file (repeatable)
  --schema <relation:column=type> Typed source column (repeatable)
  --dialect <name>                SQL dialect (default: generic)

DBT INPUT
  --dbt-project <dir>             Read target/manifest.json in a dbt project
  --dbt-manifest <path>           Read a dbt manifest.json artifact
  --dbt-catalog <path>            Require a specific catalog.json file

GENERATION OPTIONS
  --target <relation>             Select a named terminal outcome
  --target-layer <layer-id>       Select an anonymous terminal outcome
  --boundary <relation>           Materialize an intermediate (repeatable)
  --assume-comparison <name>      Attest comparison semantics (repeatable)
  --matching <n>                  Matching rows per relation (default: 100)
  --rejected <n>                  Rejected rows per relation (default: 10)
  --seed <n>                      Deterministic seed (default: 42)

OUTPUT OPTIONS
  --format <parquet|csv>          Export format (default: parquet)
  --output <dir>                  Output directory (default: sql-tdg-output)
  --name <name>                   Stable workload name stored in metadata

GENERAL
  -h, --help                      Show help (also: generate --help)
  -V, --version                   Show package version

EXAMPLES
  sql-tdg generate --sql 'SELECT amount FROM orders' --schema orders:amount=INT
  sql-tdg generate --dbt-project . --format csv --output generated
  sql-tdg generate --dbt-manifest target/manifest.json --matching 50

NOTES
  Choose raw --sql/--file inputs or dbt artifacts, not both.
  Raw SQL requires --schema; dbt uses artifact source schemas and types.
  Without --target or --target-layer, one dataset covers all terminal outcomes.
  Without --boundary, physical source relations are generated.
  --assume-comparison: binary_collation, no_char_padding, no_nan,
                       signed_zero_equivalent, session_time_zone
  dbt reads a neighboring catalog.json when present; otherwise complete
  manifest-declared source types are required, including FK-only sources.
  The CLI writes data and metadata files; it never modifies databases.
  More: https://github.com/phdah/sql-tdg#cli-generation
"#;

fn render_help(color: bool) -> String {
    if !color {
        return HELP.to_owned();
    }

    let mut rendered = String::with_capacity(HELP.len() + 256);
    for line in HELP.lines() {
        if line == "sql-tdg" {
            rendered.push_str("\x1b[1m");
            rendered.push_str(line);
            rendered.push_str("\x1b[0m");
        } else if matches!(
            line,
            "USAGE"
                | "RAW SQL INPUT"
                | "DBT INPUT"
                | "GENERATION OPTIONS"
                | "OUTPUT OPTIONS"
                | "GENERAL"
                | "EXAMPLES"
                | "NOTES"
        ) {
            rendered.push_str("\x1b[1;36m");
            rendered.push_str(line);
            rendered.push_str("\x1b[0m");
        } else if line.starts_with("  -") {
            if let Some(index) = line[2..].find("  ") {
                let end = index + 2;
                rendered.push_str("\x1b[32m");
                rendered.push_str(&line[..end]);
                rendered.push_str("\x1b[0m");
                rendered.push_str(&line[end..]);
            } else {
                rendered.push_str(line);
            }
        } else {
            rendered.push_str(line);
        }
        rendered.push('\n');
    }
    rendered
}

fn help_color_enabled(is_terminal: bool, no_color: bool, term: Option<&str>) -> bool {
    is_terminal && !no_color && term != Some("dumb")
}

fn print_help() {
    let term = env::var("TERM").ok();
    let color = help_color_enabled(
        io::stdout().is_terminal(),
        env::var_os("NO_COLOR").is_some(),
        term.as_deref(),
    );
    print!("{}", render_help(color));
}

#[derive(Debug)]
struct CliError(String);

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CliError {}

#[derive(Debug, Clone)]
enum RawInput {
    Inline(String),
    File(PathBuf),
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Csv,
    Parquet,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "csv" => Ok(Self::Csv),
            "parquet" => Ok(Self::Parquet),
            other => Err(CliError::new(format!(
                "unsupported output format {other:?}; expected csv or parquet"
            ))),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Parquet => "parquet",
        }
    }

    const fn extension(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug)]
struct GenerateArgs {
    raw_inputs: Vec<RawInput>,
    schema_specs: Vec<String>,
    dialect: Option<String>,
    dbt_project: Option<PathBuf>,
    dbt_manifest: Option<PathBuf>,
    dbt_catalog: Option<PathBuf>,
    target_relation: Option<String>,
    target_layer: Option<String>,
    boundaries: Vec<String>,
    comparison_assumptions: Vec<ComparisonAssumption>,
    seed: u64,
    matching: usize,
    rejected: usize,
    output_format: OutputFormat,
    output_dir: PathBuf,
    workload_name: Option<String>,
}

struct AnalysisProduct {
    bundle: AnalysisBundle,
    dialect: String,
    workload: WorkloadIdentity,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), CliError> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let Some(first) = arguments.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };

    match first {
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        "-V" | "--version" => {
            println!("sql-tdg {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "generate" => {
            let args = parse_generate_args(arguments.into_iter().skip(1))?;
            generate(args)
        }
        other => Err(CliError::new(format!(
            "unknown command {other:?}; expected generate"
        ))),
    }
}

fn parse_generate_args<I>(mut arguments: I) -> Result<GenerateArgs, CliError>
where
    I: Iterator<Item = String>,
{
    let mut args = GenerateArgs {
        raw_inputs: Vec::new(),
        schema_specs: Vec::new(),
        dialect: None,
        dbt_project: None,
        dbt_manifest: None,
        dbt_catalog: None,
        target_relation: None,
        target_layer: None,
        boundaries: Vec::new(),
        comparison_assumptions: Vec::new(),
        seed: 42,
        matching: 100,
        rejected: 10,
        output_format: OutputFormat::Parquet,
        output_dir: PathBuf::from("sql-tdg-output"),
        workload_name: None,
    };

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--sql" => args
                .raw_inputs
                .push(RawInput::Inline(next_value(&mut arguments, "--sql")?)),
            "--file" => args
                .raw_inputs
                .push(RawInput::File(PathBuf::from(next_value(
                    &mut arguments,
                    "--file",
                )?))),
            "--schema" => args
                .schema_specs
                .push(next_value(&mut arguments, "--schema")?),
            "--dialect" => args.dialect = Some(next_value(&mut arguments, "--dialect")?),
            "--dbt-project" => {
                args.dbt_project = Some(PathBuf::from(next_value(&mut arguments, "--dbt-project")?))
            }
            "--dbt-manifest" => {
                args.dbt_manifest =
                    Some(PathBuf::from(next_value(&mut arguments, "--dbt-manifest")?))
            }
            "--dbt-catalog" => {
                args.dbt_catalog = Some(PathBuf::from(next_value(&mut arguments, "--dbt-catalog")?))
            }
            "--target" => args.target_relation = Some(next_value(&mut arguments, "--target")?),
            "--target-layer" => {
                args.target_layer = Some(next_value(&mut arguments, "--target-layer")?)
            }
            "--boundary" => args
                .boundaries
                .push(next_value(&mut arguments, "--boundary")?),
            "--assume-comparison" => {
                let value = next_value(&mut arguments, "--assume-comparison")?;
                let assumption = ComparisonAssumption::from_name(&value).ok_or_else(|| {
                    CliError::new(format!("unsupported comparison assumption {value:?}"))
                })?;
                args.comparison_assumptions.push(assumption);
            }
            "--seed" => args.seed = parse_number(next_value(&mut arguments, "--seed")?, "--seed")?,
            "--matching" => {
                args.matching =
                    parse_number(next_value(&mut arguments, "--matching")?, "--matching")?
            }
            "--rejected" => {
                args.rejected =
                    parse_number(next_value(&mut arguments, "--rejected")?, "--rejected")?
            }
            "--format" => {
                args.output_format = OutputFormat::parse(&next_value(&mut arguments, "--format")?)?
            }
            "--output" => args.output_dir = PathBuf::from(next_value(&mut arguments, "--output")?),
            "--name" => args.workload_name = Some(next_value(&mut arguments, "--name")?),
            other => {
                return Err(CliError::new(format!(
                    "unknown generate option {other:?}; run sql-tdg --help"
                )));
            }
        }
    }

    validate_generate_args(&args)?;
    Ok(args)
}

fn next_value<I>(arguments: &mut I, option: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    arguments
        .next()
        .ok_or_else(|| CliError::new(format!("{option} requires a value")))
}

fn parse_number<T>(value: String, option: &str) -> Result<T, CliError>
where
    T: std::str::FromStr,
    T::Err: fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| CliError::new(format!("invalid value for {option}: {error}")))
}

fn validate_generate_args(args: &GenerateArgs) -> Result<(), CliError> {
    let has_raw = !args.raw_inputs.is_empty();
    let has_dbt = args.dbt_project.is_some() || args.dbt_manifest.is_some();

    if has_raw == has_dbt {
        return Err(CliError::new(
            "choose exactly one input mode: raw --sql/--file inputs or dbt --dbt-project/--dbt-manifest",
        ));
    }
    if args.dbt_project.is_some() && args.dbt_manifest.is_some() {
        return Err(CliError::new(
            "--dbt-project and --dbt-manifest are mutually exclusive",
        ));
    }
    if args.target_relation.is_some() && args.target_layer.is_some() {
        return Err(CliError::new(
            "--target and --target-layer are mutually exclusive",
        ));
    }

    if has_raw {
        if args.schema_specs.is_empty() {
            return Err(CliError::new(
                "raw SQL generation requires at least one --schema RELATION:COLUMN=TYPE entry",
            ));
        }
        if args.dbt_catalog.is_some() {
            return Err(CliError::new("--dbt-catalog is only valid with dbt input"));
        }
    } else {
        if !args.schema_specs.is_empty() {
            return Err(CliError::new(
                "--schema is only valid with raw SQL input; dbt schemas come from catalog.json or manifest column data_type declarations",
            ));
        }
        if args.dialect.is_some() {
            return Err(CliError::new(
                "--dialect is only valid with raw SQL input; dbt uses manifest metadata",
            ));
        }
    }

    Ok(())
}

fn generate(args: GenerateArgs) -> Result<(), CliError> {
    let selector = selected_outcome(&args);
    let boundary = if args.boundaries.is_empty() {
        GenerationBoundary::physical_sources()
    } else {
        GenerationBoundary::intermediate_relations(args.boundaries.clone())
            .map_err(|error| CliError::new(error.to_string()))?
    };
    let row_counts = GenerationRowCounts::new(args.matching, args.rejected)
        .map_err(|error| CliError::new(error.to_string()))?;

    let mut analysis = if args.raw_inputs.is_empty() {
        analyze_dbt(&args)?
    } else {
        analyze_raw(&args)?
    };

    analysis
        .bundle
        .declare_comparison_assumptions(&args.comparison_assumptions);

    let generated = generate_classified_from_bundle_at_boundary(
        &analysis.bundle,
        selector.as_ref(),
        &boundary,
        row_counts,
        args.seed,
    )
    .map_err(|error| CliError::new(error.to_string()))?;

    let target = target_for_metadata(&analysis.bundle, selector.as_ref())?;
    let target_label = describe_target(&target);
    let metadata = build_metadata(&analysis, target, boundary, args.seed, &generated)?;
    let exported = write_generated_outputs(&generated, args.output_format, &args.output_dir)?;
    let metadata_path = args.output_dir.join("metadata.sqltdg");
    fs::write(&metadata_path, metadata.serialize()).map_err(|error| {
        CliError::new(format!(
            "failed to write metadata {}: {error}",
            metadata_path.display()
        ))
    })?;

    println!("target={target_label}");
    println!("dialect={}", analysis.dialect);
    println!("seed={}", args.seed);
    println!("matching_rows={}", row_counts.matching());
    println!("rejected_rows={}", row_counts.rejected());
    println!("format={}", args.output_format.as_str());
    for (relation, path) in exported {
        println!("relation={relation} path={}", path.display());
    }
    for finding in generated.case_coverage() {
        println!(
            "case_branch={} status={} detail={}",
            finding.location(),
            finding.status().as_str(),
            finding.detail()
        );
    }
    for relation in generated.unhonored_constraints() {
        println!(
            "relation_constraints={relation} status=not_honored reason=relation_not_generated"
        );
    }
    println!("metadata={}", metadata_path.display());

    Ok(())
}

fn selected_outcome(args: &GenerateArgs) -> Option<OutcomeSelector> {
    if let Some(relation) = &args.target_relation {
        Some(OutcomeSelector::Relation(relation.clone()))
    } else {
        args.target_layer
            .as_ref()
            .map(|layer| OutcomeSelector::AnonymousLayer(layer.clone()))
    }
}

fn analyze_raw(args: &GenerateArgs) -> Result<AnalysisProduct, CliError> {
    let dialect_name = args.dialect.as_deref().unwrap_or("generic");
    let dialect = dialect_from_name(dialect_name)
        .ok_or_else(|| CliError::new(format!("unsupported SQL dialect {dialect_name:?}")))?;
    let schemas = parse_schemas(&args.schema_specs, dialect_name)?;
    let catalog = RelationCatalog::from_schemas(&schemas)
        .map_err(|error| CliError::new(format!("invalid source schema metadata: {error}")))?;

    let mut inputs = Vec::with_capacity(args.raw_inputs.len());
    let mut entrypoints = Vec::with_capacity(args.raw_inputs.len());
    for input in &args.raw_inputs {
        match input {
            RawInput::Inline(sql) => {
                inputs.push(SqlInput::inline(sql.clone()));
                entrypoints.push("inline".to_owned());
            }
            RawInput::File(path) => {
                let sql = fs::read_to_string(path).map_err(|error| {
                    CliError::new(format!(
                        "failed to read SQL file {}: {error}",
                        path.display()
                    ))
                })?;
                inputs.push(SqlInput::file(path.display().to_string(), sql));
                entrypoints.push(path.display().to_string());
            }
        }
    }

    let ids = (1..=inputs.len())
        .map(|index| format!("input-{index:04}"))
        .collect::<Vec<_>>();
    let configured = ids
        .iter()
        .zip(inputs.iter())
        .map(|(id, input)| ConfiguredSqlInput::new(id, input, dialect_name, dialect.as_ref()))
        .collect::<Vec<_>>();
    let bundle = analyze_configured_inputs_with_catalog(&configured, &catalog)
        .map_err(|error| CliError::new(format!("protocol analysis failed: {error}")))?;

    let workload_name = args
        .workload_name
        .clone()
        .unwrap_or_else(|| "raw-sql".to_owned());
    let entrypoint = if entrypoints.len() == 1 {
        entrypoints.remove(0)
    } else {
        format!("{} inputs", entrypoints.len())
    };
    let workload = WorkloadIdentity::raw_sql(workload_name, entrypoint)
        .map_err(|error| CliError::new(error.to_string()))?;

    Ok(AnalysisProduct {
        bundle,
        dialect: dialect_name.to_owned(),
        workload,
    })
}

fn analyze_dbt(args: &GenerateArgs) -> Result<AnalysisProduct, CliError> {
    let (manifest_path, catalog_path, default_name) = resolve_dbt_paths(args)?;
    let manifest_json = fs::read_to_string(&manifest_path).map_err(|error| {
        CliError::new(format!(
            "failed to read dbt manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    let catalog_json = match fs::read_to_string(&catalog_path) {
        Ok(json) => Some(json),
        Err(error) if args.dbt_catalog.is_none() && error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(CliError::new(format!(
                "failed to read dbt catalog {}: {error}",
                catalog_path.display()
            )));
        }
    };

    let manifest =
        parse_dbt_manifest(&manifest_json).map_err(|error| CliError::new(error.to_string()))?;
    let dialect_name = manifest.adapter_type().to_owned();
    let dialect = dialect_from_name(&dialect_name).ok_or_else(|| {
        CliError::new(format!(
            "dbt adapter type {:?} is not a supported SQL dialect",
            manifest.adapter_type()
        ))
    })?;
    let bundle = match catalog_json {
        Some(json) => {
            let catalog =
                parse_dbt_catalog(&json).map_err(|error| CliError::new(error.to_string()))?;
            analyze_dbt_artifacts(&manifest, &catalog, &dialect_name, dialect.as_ref())
        }
        None => analyze_dbt_manifest_with_schemas(&manifest, &dialect_name, dialect.as_ref()),
    }
    .map_err(|error| dbt_analysis_error(&error))?;

    let workload_name = args.workload_name.clone().unwrap_or(default_name);
    let workload =
        WorkloadIdentity::dbt_project(workload_name, manifest_path.display().to_string())
            .map_err(|error| CliError::new(error.to_string()))?;

    Ok(AnalysisProduct {
        bundle,
        dialect: dialect_name,
        workload,
    })
}

fn dbt_analysis_error(error: &DbtArtifactsError) -> CliError {
    let hint = match error {
        DbtArtifactsError::MissingCatalogSchema { .. } => {
            "\nhint: declare source columns with data_type in dbt YAML and rerun `dbt compile`; \
             alternatively populate the source tables and run `dbt docs generate` to supply \
             catalog.json"
        }
        DbtArtifactsError::MissingDeclaredColumnTypes { .. } => {
            "\nhint: add data_type to each listed source column in dbt YAML and rerun `dbt compile`, \
             or provide a catalog.json containing the relation schema"
        }
        _ => "",
    };
    CliError::new(format!("dbt protocol analysis failed: {error}{hint}"))
}

fn resolve_dbt_paths(args: &GenerateArgs) -> Result<(PathBuf, PathBuf, String), CliError> {
    if let Some(project) = &args.dbt_project {
        let manifest = project.join("target").join("manifest.json");
        let catalog = args
            .dbt_catalog
            .clone()
            .unwrap_or_else(|| project.join("target").join("catalog.json"));
        let name = project
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("dbt-project")
            .to_owned();
        return Ok((manifest, catalog, name));
    }

    let manifest = args
        .dbt_manifest
        .clone()
        .ok_or_else(|| CliError::new("dbt input requires --dbt-project or --dbt-manifest"))?;
    let catalog = args.dbt_catalog.clone().unwrap_or_else(|| {
        manifest
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("catalog.json")
    });
    let name = manifest
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("dbt-project")
        .to_owned();
    Ok((manifest, catalog, name))
}

fn parse_schemas(specs: &[String], dialect: &str) -> Result<Vec<RelationSchema>, CliError> {
    let mut columns_by_relation = BTreeMap::<String, Vec<SchemaColumn>>::new();

    for spec in specs {
        let (relation, column_and_type) = spec.split_once(':').ok_or_else(|| {
            CliError::new(format!(
                "invalid --schema {spec:?}; expected RELATION:COLUMN=TYPE"
            ))
        })?;
        let (column, sql_type) = column_and_type.split_once('=').ok_or_else(|| {
            CliError::new(format!(
                "invalid --schema {spec:?}; expected RELATION:COLUMN=TYPE"
            ))
        })?;
        let relation = relation.trim();
        let column = column.trim();
        let sql_type = sql_type.trim();
        if relation.is_empty() || column.is_empty() || sql_type.is_empty() {
            return Err(CliError::new(format!(
                "invalid --schema {spec:?}; relation, column, and type must be non-empty"
            )));
        }

        let schema_column =
            SchemaColumn::from_sql_type(column, sql_type, dialect).map_err(|error| {
                CliError::new(format!("invalid datatype for {relation}.{column}: {error}"))
            })?;
        columns_by_relation
            .entry(relation.to_owned())
            .or_default()
            .push(schema_column);
    }

    columns_by_relation
        .into_iter()
        .map(|(relation, columns)| {
            RelationSchema::new(relation, columns)
                .map_err(|error| CliError::new(format!("invalid relation schema: {error}")))
        })
        .collect()
}

fn target_for_metadata(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
) -> Result<TestTarget, CliError> {
    match selector {
        Some(OutcomeSelector::Relation(relation)) => {
            TestTarget::relation(relation.clone()).map_err(|error| CliError::new(error.to_string()))
        }
        Some(OutcomeSelector::AnonymousLayer(layer)) => TestTarget::anonymous_layer(layer.clone())
            .map_err(|error| CliError::new(error.to_string())),
        None => {
            let outcomes = bundle
                .graph()
                .components()
                .iter()
                .flat_map(|component| component.final_outcomes())
                .collect::<Vec<_>>();
            match outcomes.as_slice() {
                [] => Err(CliError::new("protocol bundle has no terminal outcome")),
                [DatasetRef::Relation { name }] => TestTarget::relation(name.clone())
                    .map_err(|error| CliError::new(error.to_string())),
                [DatasetRef::Anonymous { layer_id }] => {
                    TestTarget::anonymous_layer(layer_id.clone())
                        .map_err(|error| CliError::new(error.to_string()))
                }
                [_] => Err(CliError::new(
                    "selected protocol outcome cannot be represented by the CLI",
                )),
                _ => {
                    let mut descriptions = outcomes
                        .iter()
                        .map(|outcome| match outcome {
                            DatasetRef::Relation { name } => Ok(format!("relation:{name}")),
                            DatasetRef::Anonymous { layer_id } => {
                                Ok(format!("anonymous:{layer_id}"))
                            }
                            _ => Err(CliError::new(
                                "terminal protocol outcome cannot be represented by the CLI",
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    descriptions.sort();
                    TestTarget::all_terminal_outcomes(&descriptions)
                        .map_err(|error| CliError::new(error.to_string()))
                }
            }
        }
    }
}

fn build_metadata(
    analysis: &AnalysisProduct,
    target: TestTarget,
    boundary: GenerationBoundary,
    seed: u64,
    generated: &GeneratedData,
) -> Result<TestCaseMetadata, CliError> {
    let counts = generated.row_counts();
    let matching = u64::try_from(counts.matching())
        .map_err(|_| CliError::new("matching row count cannot be represented as u64"))?;
    let rejected = u64::try_from(counts.rejected())
        .map_err(|_| CliError::new("rejected row count cannot be represented as u64"))?;
    let relations = generated
        .tables()
        .keys()
        .map(|relation| {
            GeneratedRelation::new(relation.clone(), matching, rejected)
                .map_err(|error| CliError::new(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let protocol = ProtocolSnapshot::new(to_bundle_json(&analysis.bundle))
        .map_err(|error| CliError::new(error.to_string()))?;

    TestCaseMetadata::new(
        analysis.workload.clone(),
        target,
        boundary,
        analysis.dialect.clone(),
        seed,
        protocol,
        relations,
    )
    .map_err(|error| CliError::new(error.to_string()))
}

fn write_generated_outputs(
    generated: &GeneratedData,
    format: OutputFormat,
    output_dir: &Path,
) -> Result<Vec<(String, PathBuf)>, CliError> {
    fs::create_dir_all(output_dir).map_err(|error| {
        CliError::new(format!(
            "failed to create output directory {}: {error}",
            output_dir.display()
        ))
    })?;

    let mut exported = Vec::with_capacity(generated.tables().len());
    for (index, (relation, table)) in generated.tables().iter().enumerate() {
        let filename = format!(
            "{:04}-{}.{}",
            index + 1,
            sanitize_relation(relation),
            format.extension()
        );
        let path = output_dir.join(filename);
        match format {
            OutputFormat::Csv => write_csv(table, &path),
            OutputFormat::Parquet => write_parquet(table, &path),
        }
        .map_err(|error| {
            CliError::new(format!(
                "failed to export relation {relation:?} to {}: {error}",
                path.display()
            ))
        })?;
        exported.push((relation.clone(), path));
    }

    Ok(exported)
}

fn sanitize_relation(relation: &str) -> String {
    let sanitized = relation
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized
        .chars()
        .any(|character| character.is_ascii_alphanumeric())
    {
        sanitized
    } else {
        "relation".to_owned()
    }
}

fn describe_target(target: &TestTarget) -> String {
    match target.kind() {
        TargetKind::Relation => format!("relation:{}", target.identifier()),
        TargetKind::AnonymousLayer => format!("anonymous:{}", target.identifier()),
        TargetKind::AllTerminalOutcomes => format!("all:{}", target.identifier()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_plain_and_within_eighty_columns() {
        let plain = render_help(false);
        assert_eq!(plain, HELP);
        assert!(!plain.contains("\x1b["));
        assert!(plain.lines().all(|line| line.len() <= 80));
        for heading in [
            "USAGE",
            "RAW SQL INPUT",
            "DBT INPUT",
            "GENERATION OPTIONS",
            "OUTPUT OPTIONS",
            "EXAMPLES",
        ] {
            assert!(plain.lines().any(|line| line == heading));
        }
        assert!(plain.contains("--assume-comparison"));
        assert!(plain.contains("--target-layer"));
    }

    #[test]
    fn colored_help_preserves_plain_content() {
        let colored = render_help(true);
        assert!(colored.contains("\x1b[1;36mUSAGE\x1b[0m"));
        assert!(colored.contains("\x1b[32m  -h, --help\x1b[0m"));
        let stripped = colored
            .replace("\x1b[1;36m", "")
            .replace("\x1b[32m", "")
            .replace("\x1b[1m", "")
            .replace("\x1b[0m", "");
        assert_eq!(stripped, HELP);
    }

    #[test]
    fn help_color_respects_terminal_and_environment() {
        assert!(help_color_enabled(true, false, Some("xterm-256color")));
        assert!(!help_color_enabled(false, false, Some("xterm-256color")));
        assert!(!help_color_enabled(true, true, Some("xterm-256color")));
        assert!(!help_color_enabled(true, false, Some("dumb")));
    }
}
