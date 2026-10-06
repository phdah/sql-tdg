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

    pub fn materialize_parquet(
        &self,
        relation: &str,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let qualified = qualified_relation(relation)?;
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| "Parquet path is not valid UTF-8".to_owned())?;

        if let Some((schema, _)) = relation.split_once('.') {
            self.connection
                .execute_batch(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {}",
                    quote_identifier(schema)
                ))
                .map_err(|error| error.to_string())?;
        }

        self.connection
            .execute_batch(&format!(
                "DROP TABLE IF EXISTS {qualified}; \
                 CREATE TABLE {qualified} AS SELECT * FROM read_parquet({})",
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
}

fn qualified_relation(relation: &str) -> Result<String, String> {
    let parts = relation.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        [table] if !table.is_empty() => Ok(quote_identifier(table)),
        [schema, table] if !schema.is_empty() && !table.is_empty() => Ok(format!(
            "{}.{}",
            quote_identifier(schema),
            quote_identifier(table)
        )),
        _ => Err(format!(
            "relation {relation:?} is not a valid DuckDB table identity"
        )),
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
