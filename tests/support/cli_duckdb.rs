use std::path::Path;

use duckdb::Connection;

/// Small DuckDB boundary for executing files produced by the public CLI.
pub struct CliDuckDb {
    connection: Connection,
}

impl CliDuckDb {
    pub fn in_memory() -> Result<Self, String> {
        Connection::open_in_memory()
            .map(|connection| Self { connection })
            .map_err(|error| error.to_string())
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Connection::open(path)
            .map(|connection| Self { connection })
            .map_err(|error| error.to_string())
    }

    pub fn materialize_parquet(
        &self,
        relation: &str,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let qualified = self.prepare_relation(relation)?;
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| "Parquet path is not valid UTF-8".to_owned())?;

        self.connection
            .execute_batch(&format!(
                "DROP TABLE IF EXISTS {qualified}; \
                 CREATE TABLE {qualified} AS SELECT * FROM read_parquet({})",
                quote_literal(path)
            ))
            .map_err(|error| error.to_string())
    }

    pub fn materialize_csv(&self, relation: &str, path: impl AsRef<Path>) -> Result<(), String> {
        let qualified = self.prepare_relation(relation)?;
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| "CSV path is not valid UTF-8".to_owned())?;

        self.connection
            .execute_batch(&format!(
                "DROP TABLE IF EXISTS {qualified}; \
                 CREATE TABLE {qualified} AS \
                 SELECT * FROM read_csv_auto({}, header = true)",
                quote_literal(path)
            ))
            .map_err(|error| error.to_string())
    }

    pub fn execute_text(
        &self,
        workload: &str,
        column_count: usize,
    ) -> Result<Vec<Vec<String>>, String> {
        let mut statement = self
            .connection
            .prepare(workload)
            .map_err(|error| error.to_string())?;
        let mut rows = statement.query([]).map_err(|error| error.to_string())?;
        let mut result = Vec::new();

        while let Some(row) = rows.next().map_err(|error| error.to_string())? {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                values.push(row.get(index).map_err(|error| error.to_string())?);
            }
            result.push(values);
        }

        Ok(result)
    }

    fn prepare_relation(&self, relation: &str) -> Result<String, String> {
        let parts = relation_parts(relation)?;
        if parts.len() >= 2 {
            let schema = &parts[parts.len() - 2];
            self.connection
                .execute_batch(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {}",
                    quote_identifier(schema)
                ))
                .map_err(|error| error.to_string())?;
        }

        Ok(parts
            .iter()
            .map(|part| quote_identifier(part))
            .collect::<Vec<_>>()
            .join("."))
    }
}

fn relation_parts(relation: &str) -> Result<Vec<String>, String> {
    let parts = relation
        .split('.')
        .map(|part| part.trim().trim_matches('"').to_owned())
        .collect::<Vec<_>>();
    if !(1..=3).contains(&parts.len()) || parts.iter().any(String::is_empty) {
        return Err(format!(
            "relation {relation:?} is not a valid DuckDB table identity"
        ));
    }
    Ok(parts)
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
