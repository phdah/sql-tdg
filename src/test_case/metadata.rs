use std::collections::BTreeSet;

use super::codec::{
    METADATA_HEADER, decode_string, encode_string, invalid_metadata, next_value, parse_count,
    parse_u64, record,
};
use super::{
    BoundaryKind, GeneratedRelation, GenerationBoundary, ProtocolSnapshot, TargetKind,
    TestCaseError, TestTarget, WorkloadIdentity, WorkloadKind, reject_duplicates, required_string,
};

/// Reproducibility metadata shared by CLI and integration-test workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseMetadata {
    generator_version: String,
    workload: WorkloadIdentity,
    target: TestTarget,
    boundary: GenerationBoundary,
    dialect: String,
    seed: u64,
    protocol: ProtocolSnapshot,
    relations: Vec<GeneratedRelation>,
}

impl TestCaseMetadata {
    /// Creates metadata tied to the current sql-tdg package version.
    pub fn new(
        workload: WorkloadIdentity,
        target: TestTarget,
        boundary: GenerationBoundary,
        dialect: impl Into<String>,
        seed: u64,
        protocol: ProtocolSnapshot,
        relations: Vec<GeneratedRelation>,
    ) -> Result<Self, TestCaseError> {
        Self::new_with_generator_version(
            env!("CARGO_PKG_VERSION"),
            workload,
            target,
            boundary,
            dialect,
            seed,
            protocol,
            relations,
        )
    }

    fn new_with_generator_version(
        generator_version: impl Into<String>,
        workload: WorkloadIdentity,
        target: TestTarget,
        boundary: GenerationBoundary,
        dialect: impl Into<String>,
        seed: u64,
        protocol: ProtocolSnapshot,
        mut relations: Vec<GeneratedRelation>,
    ) -> Result<Self, TestCaseError> {
        let generator_version = required_string(generator_version, "generator version")?;
        let dialect = required_string(dialect, "dialect")?;
        if relations.is_empty() {
            return Err(TestCaseError::EmptyCollection {
                field: "generated relations",
            });
        }

        relations.sort_by(|left, right| left.relation().cmp(right.relation()));
        let relation_names = relations
            .iter()
            .map(|relation| relation.relation().to_owned())
            .collect::<Vec<_>>();
        reject_duplicates(&relation_names)?;

        if boundary.kind() == BoundaryKind::IntermediateRelations {
            let generated = relation_names
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for relation in boundary.relations() {
                if !generated.contains(relation.as_str()) {
                    return Err(TestCaseError::MissingBoundaryRelation {
                        relation: relation.clone(),
                    });
                }
            }
        }

        Ok(Self {
            generator_version,
            workload,
            target,
            boundary,
            dialect,
            seed,
            protocol,
            relations,
        })
    }

    /// Returns the sql-tdg version that created the test case.
    pub fn generator_version(&self) -> &str {
        &self.generator_version
    }

    /// Returns the stable workload identity.
    pub fn workload(&self) -> &WorkloadIdentity {
        &self.workload
    }

    /// Returns the explicitly selected protocol target.
    pub fn target(&self) -> &TestTarget {
        &self.target
    }

    /// Returns the selected generation boundary.
    pub fn boundary(&self) -> &GenerationBoundary {
        &self.boundary
    }

    /// Returns the SQL dialect consumed by SQL Semantic Protocol.
    pub fn dialect(&self) -> &str {
        &self.dialect
    }

    /// Returns the deterministic generation seed.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the opaque normalized protocol snapshot used for generation.
    pub fn protocol(&self) -> &ProtocolSnapshot {
        &self.protocol
    }

    /// Returns generated relations in deterministic identity order.
    pub fn relations(&self) -> &[GeneratedRelation] {
        &self.relations
    }

    /// Serializes reproducibility metadata to the stable v1 line-oriented contract.
    pub fn serialize(&self) -> String {
        let mut records = vec![METADATA_HEADER.to_owned()];
        records.push(record(
            "generator_version",
            &encode_string(&self.generator_version),
        ));
        records.push(record("workload_kind", self.workload.kind().as_str()));
        records.push(record(
            "workload_name",
            &encode_string(self.workload.name()),
        ));
        records.push(record(
            "workload_entrypoint",
            &encode_string(self.workload.entrypoint()),
        ));
        records.push(record("target_kind", self.target.kind().as_str()));
        records.push(record(
            "target_identifier",
            &encode_string(self.target.identifier()),
        ));
        records.push(record("boundary_kind", self.boundary.kind().as_str()));
        records.push(record(
            "boundary_count",
            &self.boundary.relations().len().to_string(),
        ));
        for relation in self.boundary.relations() {
            records.push(record("boundary_relation", &encode_string(relation)));
        }
        records.push(record("dialect", &encode_string(&self.dialect)));
        records.push(record("seed", &self.seed.to_string()));
        records.push(record(
            "protocol",
            &encode_string(self.protocol.document()),
        ));
        records.push(record("relation_count", &self.relations.len().to_string()));
        for relation in &self.relations {
            records.push(format!(
                "relation\t{}\t{}\t{}",
                encode_string(relation.relation()),
                relation.rows().matching(),
                relation.rows().rejected()
            ));
        }
        records.join("\n")
    }

    /// Deserializes metadata produced by serialize.
    pub fn deserialize(serialized: &str) -> Result<Self, TestCaseError> {
        let mut lines = serialized.lines();
        let header = lines
            .next()
            .ok_or_else(|| invalid_metadata("metadata is empty"))?;
        if header != METADATA_HEADER {
            return Err(invalid_metadata(format!(
                "unsupported metadata header {header:?}"
            )));
        }

        let generator_version = decode_string(next_value(&mut lines, "generator_version")?)?;
        let workload_kind = WorkloadKind::parse(next_value(&mut lines, "workload_kind")?)?;
        let workload_name = decode_string(next_value(&mut lines, "workload_name")?)?;
        let workload_entrypoint = decode_string(next_value(&mut lines, "workload_entrypoint")?)?;
        let workload = WorkloadIdentity::new(workload_kind, workload_name, workload_entrypoint)?;

        let target_kind = TargetKind::parse(next_value(&mut lines, "target_kind")?)?;
        let target_identifier = decode_string(next_value(&mut lines, "target_identifier")?)?;
        let target = TestTarget::new(target_kind, target_identifier)?;

        let boundary_kind = BoundaryKind::parse(next_value(&mut lines, "boundary_kind")?)?;
        let boundary_count = parse_count(
            next_value(&mut lines, "boundary_count")?,
            "boundary_count",
        )?;
        let mut boundary_relations = Vec::with_capacity(boundary_count);
        for _ in 0..boundary_count {
            boundary_relations.push(decode_string(next_value(
                &mut lines,
                "boundary_relation",
            )?)?);
        }
        let boundary = match boundary_kind {
            BoundaryKind::PhysicalSources => {
                if !boundary_relations.is_empty() {
                    return Err(invalid_metadata(
                        "physical-source boundary cannot declare intermediate relations",
                    ));
                }
                GenerationBoundary::physical_sources()
            }
            BoundaryKind::IntermediateRelations => {
                GenerationBoundary::intermediate_relations(boundary_relations)?
            }
        };

        let dialect = decode_string(next_value(&mut lines, "dialect")?)?;
        let seed = parse_u64(next_value(&mut lines, "seed")?, "seed")?;
        let protocol = ProtocolSnapshot::new(decode_string(next_value(&mut lines, "protocol")?)?)?;
        let relation_count = parse_count(
            next_value(&mut lines, "relation_count")?,
            "relation_count",
        )?;
        let mut relations = Vec::with_capacity(relation_count);
        for _ in 0..relation_count {
            let line = lines
                .next()
                .ok_or_else(|| invalid_metadata("missing relation record"))?;
            let mut fields = line.split('\t');
            let key = fields
                .next()
                .ok_or_else(|| invalid_metadata("empty relation record"))?;
            let relation = fields
                .next()
                .ok_or_else(|| invalid_metadata("relation record has no identity"))?;
            let matching = fields
                .next()
                .ok_or_else(|| invalid_metadata("relation record has no matching row count"))?;
            let rejected = fields
                .next()
                .ok_or_else(|| invalid_metadata("relation record has no rejected row count"))?;
            if key != "relation" || fields.next().is_some() {
                return Err(invalid_metadata(format!(
                    "invalid relation record {line:?}"
                )));
            }
            relations.push(GeneratedRelation::new(
                decode_string(relation)?,
                parse_u64(matching, "matching rows")?,
                parse_u64(rejected, "rejected rows")?,
            )?);
        }

        if let Some(line) = lines.next() {
            return Err(invalid_metadata(format!(
                "unexpected trailing metadata record {line:?}"
            )));
        }

        Self::new_with_generator_version(
            generator_version,
            workload,
            target,
            boundary,
            dialect,
            seed,
            protocol,
            relations,
        )
    }
}
