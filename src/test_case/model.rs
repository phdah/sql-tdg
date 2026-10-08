use super::{TestCaseError, reject_duplicates, required_string};

/// Kind of workload exercised by a test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    /// One or more raw SQL statements.
    RawSql,
    /// A dbt Core project workload.
    DbtProject,
}

impl WorkloadKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::RawSql => "raw-sql",
            Self::DbtProject => "dbt-project",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, TestCaseError> {
        match value {
            "raw-sql" => Ok(Self::RawSql),
            "dbt-project" => Ok(Self::DbtProject),
            _ => Err(TestCaseError::InvalidMetadata {
                message: format!("unknown workload kind {value:?}"),
            }),
        }
    }
}

/// Stable identity for the workload whose current implementation is verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadIdentity {
    kind: WorkloadKind,
    name: String,
    entrypoint: String,
}

impl WorkloadIdentity {
    /// Identifies a raw SQL workload without embedding its current query text.
    pub fn raw_sql(
        name: impl Into<String>,
        entrypoint: impl Into<String>,
    ) -> Result<Self, TestCaseError> {
        Self::new(WorkloadKind::RawSql, name, entrypoint)
    }

    /// Identifies a dbt project workload and its caller-defined entrypoint or selector.
    pub fn dbt_project(
        name: impl Into<String>,
        entrypoint: impl Into<String>,
    ) -> Result<Self, TestCaseError> {
        Self::new(WorkloadKind::DbtProject, name, entrypoint)
    }

    pub(super) fn new(
        kind: WorkloadKind,
        name: impl Into<String>,
        entrypoint: impl Into<String>,
    ) -> Result<Self, TestCaseError> {
        Ok(Self {
            kind,
            name: required_string(name, "workload name")?,
            entrypoint: required_string(entrypoint, "workload entrypoint")?,
        })
    }

    /// Returns the workload category.
    pub const fn kind(&self) -> WorkloadKind {
        self.kind
    }

    /// Returns the stable workload name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the file, model selector, or other caller-defined workload entrypoint.
    pub fn entrypoint(&self) -> &str {
        &self.entrypoint
    }
}

/// Kind of terminal outcome selected from the protocol graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A named relation produced by a terminal layer.
    Relation,
    /// An anonymous terminal layer selected by its protocol layer identifier.
    AnonymousLayer,
    /// Every terminal outcome, generated together as one shared source dataset.
    AllTerminalOutcomes,
    /// A compatible subset of terminal outcomes generated as one independent scenario.
    ScenarioOutcomes,
}

impl TargetKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Relation => "relation",
            Self::AnonymousLayer => "anonymous-layer",
            Self::AllTerminalOutcomes => "all-terminal-outcomes",
            Self::ScenarioOutcomes => "scenario-outcomes",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, TestCaseError> {
        match value {
            "relation" => Ok(Self::Relation),
            "anonymous-layer" => Ok(Self::AnonymousLayer),
            "all-terminal-outcomes" => Ok(Self::AllTerminalOutcomes),
            "scenario-outcomes" => Ok(Self::ScenarioOutcomes),
            _ => Err(TestCaseError::InvalidMetadata {
                message: format!("unknown target kind {value:?}"),
            }),
        }
    }
}

/// Explicit terminal target selected for generation and verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTarget {
    kind: TargetKind,
    identifier: String,
}

impl TestTarget {
    /// Selects a named terminal relation.
    pub fn relation(identifier: impl Into<String>) -> Result<Self, TestCaseError> {
        Self::new(TargetKind::Relation, identifier)
    }

    /// Selects an anonymous terminal layer.
    pub fn anonymous_layer(identifier: impl Into<String>) -> Result<Self, TestCaseError> {
        Self::new(TargetKind::AnonymousLayer, identifier)
    }

    /// Selects every terminal outcome, identified by their joined stable descriptions.
    pub fn all_terminal_outcomes(outcomes: &[String]) -> Result<Self, TestCaseError> {
        Self::new(TargetKind::AllTerminalOutcomes, outcomes.join(", "))
    }

    /// Selects the exact terminal outcome identities covered by one generated scenario.
    pub fn scenario_outcomes(outcomes: &[String]) -> Result<Self, TestCaseError> {
        if outcomes.is_empty() {
            return Err(TestCaseError::EmptyCollection {
                field: "scenario outcomes",
            });
        }
        Self::new(TargetKind::ScenarioOutcomes, outcomes.join(", "))
    }

    pub(super) fn new(
        kind: TargetKind,
        identifier: impl Into<String>,
    ) -> Result<Self, TestCaseError> {
        Ok(Self {
            kind,
            identifier: required_string(identifier, "target identifier")?,
        })
    }

    /// Returns how the target is identified.
    pub const fn kind(&self) -> TargetKind {
        self.kind
    }

    /// Returns the protocol relation or layer identifier, or the joined outcome descriptions
    /// for [`TargetKind::AllTerminalOutcomes`] or [`TargetKind::ScenarioOutcomes`].
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}

/// Kind of relation boundary materialized by a test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Materialize the physical leaf/source relations required by the selected target.
    PhysicalSources,
    /// Materialize one or more selected intermediate produced relations.
    IntermediateRelations,
}

impl BoundaryKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::PhysicalSources => "physical-sources",
            Self::IntermediateRelations => "intermediate-relations",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, TestCaseError> {
        match value {
            "physical-sources" => Ok(Self::PhysicalSources),
            "intermediate-relations" => Ok(Self::IntermediateRelations),
            _ => Err(TestCaseError::InvalidMetadata {
                message: format!("unknown generation boundary kind {value:?}"),
            }),
        }
    }
}

/// Explicit point in the transformation graph where generated relations are materialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationBoundary {
    kind: BoundaryKind,
    relations: Vec<String>,
}

impl GenerationBoundary {
    /// Generates the physical source relations required by the selected target.
    pub const fn physical_sources() -> Self {
        Self {
            kind: BoundaryKind::PhysicalSources,
            relations: Vec::new(),
        }
    }

    /// Generates explicitly selected intermediate produced relations.
    pub fn intermediate_relations<I, S>(relations: I) -> Result<Self, TestCaseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut relations = relations
            .into_iter()
            .map(|relation| required_string(relation, "intermediate relation"))
            .collect::<Result<Vec<_>, _>>()?;
        if relations.is_empty() {
            return Err(TestCaseError::EmptyCollection {
                field: "intermediate relations",
            });
        }
        relations.sort();
        reject_duplicates(&relations)?;

        Ok(Self {
            kind: BoundaryKind::IntermediateRelations,
            relations,
        })
    }

    /// Returns the boundary category.
    pub const fn kind(&self) -> BoundaryKind {
        self.kind
    }

    /// Returns selected intermediate relations in deterministic order.
    pub fn relations(&self) -> &[String] {
        &self.relations
    }
}

/// Matching and deliberately rejected row counts for one generated relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassifiedRowCounts {
    matching: u64,
    rejected: u64,
}

impl ClassifiedRowCounts {
    /// Creates explicit matching and rejected row counts.
    pub const fn new(matching: u64, rejected: u64) -> Self {
        Self { matching, rejected }
    }

    /// Returns the number of rows intended to satisfy the selected semantics.
    pub const fn matching(self) -> u64 {
        self.matching
    }

    /// Returns the number of rows deliberately generated outside selected semantics.
    pub const fn rejected(self) -> u64 {
        self.rejected
    }
}

/// One relation generated as part of a reproducible test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedRelation {
    relation: String,
    rows: ClassifiedRowCounts,
}

impl GeneratedRelation {
    /// Records a generated relation and its positive/negative row classification.
    pub fn new(
        relation: impl Into<String>,
        matching_rows: u64,
        rejected_rows: u64,
    ) -> Result<Self, TestCaseError> {
        Ok(Self {
            relation: required_string(relation, "generated relation")?,
            rows: ClassifiedRowCounts::new(matching_rows, rejected_rows),
        })
    }

    /// Returns the exact protocol relation identity.
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Returns the classified row counts.
    pub const fn rows(&self) -> ClassifiedRowCounts {
        self.rows
    }
}

/// Opaque normalized SQL Semantic Protocol document used to reproduce generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSnapshot {
    document: String,
}

impl ProtocolSnapshot {
    /// Stores the normalized protocol document without interpreting or rewriting it.
    pub fn new(document: impl Into<String>) -> Result<Self, TestCaseError> {
        Ok(Self {
            document: required_string(document, "protocol document")?,
        })
    }

    /// Returns the exact normalized protocol document supplied by the caller.
    pub fn document(&self) -> &str {
        &self.document
    }
}
