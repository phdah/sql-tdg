//! SQL parsing and lowering into project-owned intermediate representations.

use sqlparser::ast::{
    BinaryOperator, Expr, GroupByExpr, Join, JoinConstraint, JoinOperator, Query, Select,
    SelectItem, SetExpr, Statement, TableFactor, Value,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests;

/// Comparison operator represented in condition IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOperator {
    /// Equality comparison.
    Equal,
    /// Inequality comparison.
    NotEqual,
    /// Less-than comparison.
    LessThan,
    /// Less-than-or-equal comparison.
    LessThanOrEqual,
    /// Greater-than comparison.
    GreaterThan,
    /// Greater-than-or-equal comparison.
    GreaterThanOrEqual,
    /// A bare boolean expression that must evaluate to true.
    Boolean,
}

impl ConditionOperator {
    /// Returns the canonical operator text used by the project IR.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::Boolean => "bool",
        }
    }
}

impl fmt::Display for ConditionOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A single lowered condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionIR {
    left: String,
    operator: ConditionOperator,
    right: String,
}

impl ConditionIR {
    fn comparison(left: String, operator: ConditionOperator, right: String) -> Self {
        Self {
            left,
            operator,
            right,
        }
    }

    fn boolean(left: String) -> Self {
        Self {
            left,
            operator: ConditionOperator::Boolean,
            right: "true".to_owned(),
        }
    }

    /// Returns the left operand.
    pub fn left(&self) -> &str {
        &self.left
    }

    /// Returns the condition operator.
    pub const fn operator(&self) -> ConditionOperator {
        self.operator
    }

    /// Returns the right operand.
    pub fn right(&self) -> &str {
        &self.right
    }
}

/// Supported join kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// Inner join, including a plain JOIN.
    Inner,
    /// Left join.
    Left,
    /// Right join.
    Right,
    /// Full join.
    Full,
    /// Cross join.
    Cross,
    /// Natural inner join.
    NaturalInner,
    /// Natural left join.
    NaturalLeft,
    /// Natural right join.
    NaturalRight,
    /// Natural full join.
    NaturalFull,
}

impl JoinKind {
    /// Returns the canonical join text used by the project IR.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inner => "INNER",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
            Self::Full => "FULL",
            Self::Cross => "CROSS",
            Self::NaturalInner => "NATURAL INNER",
            Self::NaturalLeft => "NATURAL LEFT",
            Self::NaturalRight => "NATURAL RIGHT",
            Self::NaturalFull => "NATURAL FULL",
        }
    }

    fn natural(self) -> Result<Self, ParserError> {
        match self {
            Self::Inner => Ok(Self::NaturalInner),
            Self::Left => Ok(Self::NaturalLeft),
            Self::Right => Ok(Self::NaturalRight),
            Self::Full => Ok(Self::NaturalFull),
            Self::Cross
            | Self::NaturalInner
            | Self::NaturalLeft
            | Self::NaturalRight
            | Self::NaturalFull => Err(ParserError::UnsupportedJoin(
                "NATURAL CROSS JOIN".to_owned(),
            )),
        }
    }
}

impl fmt::Display for JoinKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Lowered representation of one join clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinIR {
    kind: JoinKind,
    table: String,
    conditions: Vec<ConditionIR>,
}

impl JoinIR {
    /// Returns the join kind.
    pub const fn kind(&self) -> JoinKind {
        self.kind
    }

    /// Returns the joined table name.
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Returns the lowered join conditions.
    pub fn conditions(&self) -> &[ConditionIR] {
        &self.conditions
    }
}

/// Parsed query represented only with project-owned types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryIR {
    projection: Vec<String>,
    source: String,
    joins: Vec<JoinIR>,
    conditions: Vec<ConditionIR>,
}

impl QueryIR {
    /// Returns the SELECT projection expressions.
    pub fn projection(&self) -> &[String] {
        &self.projection
    }

    /// Returns the FROM table name.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns all joins in source order.
    pub fn joins(&self) -> &[JoinIR] {
        &self.joins
    }

    /// Returns WHERE conditions followed by QUALIFY conditions.
    pub fn conditions(&self) -> &[ConditionIR] {
        &self.conditions
    }
}

/// Errors returned while parsing or lowering SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    /// The SQL parser rejected the input.
    Syntax(String),
    /// Exactly one SQL statement is required.
    ExpectedSingleStatement {
        /// Number of parsed statements.
        count: usize,
    },
    /// The statement is not a SELECT query.
    UnsupportedStatement(String),
    /// The query contains a clause outside the supported grammar.
    UnsupportedQuery(String),
    /// A projection expression cannot be represented safely.
    UnsupportedProjection(String),
    /// A table reference cannot be represented safely.
    UnsupportedTable(String),
    /// A join kind cannot be represented safely.
    UnsupportedJoin(String),
    /// A join constraint cannot be represented safely.
    UnsupportedJoinConstraint(String),
    /// An expression cannot be represented safely.
    UnsupportedExpression(String),
    /// A lexical construct is outside the supported grammar.
    UnsupportedSyntax(String),
}

impl fmt::Display for ParserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(message) => write!(formatter, "could not parse SQL: {message}"),
            Self::ExpectedSingleStatement { count } => {
                write!(formatter, "expected one SQL statement, got {count}")
            }
            Self::UnsupportedStatement(statement) => {
                write!(formatter, "unsupported SQL statement: {statement}")
            }
            Self::UnsupportedQuery(clause) => {
                write!(formatter, "unsupported query clause: {clause}")
            }
            Self::UnsupportedProjection(projection) => {
                write!(formatter, "unsupported projection: {projection}")
            }
            Self::UnsupportedTable(table) => {
                write!(formatter, "unsupported table reference: {table}")
            }
            Self::UnsupportedJoin(join) => write!(formatter, "unsupported join: {join}"),
            Self::UnsupportedJoinConstraint(constraint) => {
                write!(formatter, "unsupported join constraint: {constraint}")
            }
            Self::UnsupportedExpression(expression) => {
                write!(formatter, "unsupported expression: {expression}")
            }
            Self::UnsupportedSyntax(syntax) => {
                write!(formatter, "unsupported SQL syntax: {syntax}")
            }
        }
    }
}

impl Error for ParserError {}

/// Parses one SELECT query and lowers it into project-owned IR.
pub fn parse_query(sql: &str) -> Result<QueryIR, ParserError> {
    let (normalized_sql, natural_joins) = normalize_natural_joins(sql)?;
    let dialect = GenericDialect {};
    let mut statements = Parser::parse_sql(&dialect, &normalized_sql)
        .map_err(|error| ParserError::Syntax(error.to_string()))?;

    if statements.len() != 1 {
        return Err(ParserError::ExpectedSingleStatement {
            count: statements.len(),
        });
    }

    let statement = statements
        .pop()
        .ok_or(ParserError::ExpectedSingleStatement { count: 0 })?;
    let Statement::Query(query) = statement else {
        return Err(ParserError::UnsupportedStatement(statement.to_string()));
    };

    lower_query(&query, &natural_joins)
}

fn lower_query(query: &Query, natural_joins: &[bool]) -> Result<QueryIR, ParserError> {
    reject_query_extensions(query)?;

    let SetExpr::Select(select) = query.body.as_ref() else {
        return Err(ParserError::UnsupportedQuery(query.body.to_string()));
    };
    reject_select_extensions(select)?;

    if select.from.len() != 1 {
        return Err(ParserError::UnsupportedQuery(format!(
            "expected one FROM source, got {}",
            select.from.len()
        )));
    }

    let from = &select.from[0];
    if natural_joins.len() != from.joins.len() {
        return Err(ParserError::UnsupportedSyntax(
            "could not associate NATURAL modifiers with joins".to_owned(),
        ));
    }

    let source = plain_table_name(&from.relation, false)?;
    let projection = select
        .projection
        .iter()
        .map(lower_projection)
        .collect::<Result<Vec<_>, _>>()?;

    let joins = from
        .joins
        .iter()
        .zip(natural_joins)
        .map(|(join, natural)| lower_join(join, *natural))
        .collect::<Result<Vec<_>, _>>()?;

    let mut conditions = Vec::new();
    if let Some(selection) = &select.selection {
        lower_conditions(selection, &mut conditions)?;
    }
    if let Some(qualify) = &select.qualify {
        lower_conditions(qualify, &mut conditions)?;
    }

    Ok(QueryIR {
        projection,
        source,
        joins,
        conditions,
    })
}

fn reject_query_extensions(query: &Query) -> Result<(), ParserError> {
    if query.with.is_some() {
        return Err(ParserError::UnsupportedQuery("WITH".to_owned()));
    }
    if query.order_by.is_some() {
        return Err(ParserError::UnsupportedQuery("ORDER BY".to_owned()));
    }
    if query.limit_clause.is_some() {
        return Err(ParserError::UnsupportedQuery("LIMIT/OFFSET".to_owned()));
    }
    if query.fetch.is_some() {
        return Err(ParserError::UnsupportedQuery("FETCH".to_owned()));
    }
    if !query.locks.is_empty() {
        return Err(ParserError::UnsupportedQuery("locking clause".to_owned()));
    }
    if query.for_clause.is_some() {
        return Err(ParserError::UnsupportedQuery("FOR clause".to_owned()));
    }
    if query.settings.is_some() {
        return Err(ParserError::UnsupportedQuery("SETTINGS".to_owned()));
    }
    if query.format_clause.is_some() {
        return Err(ParserError::UnsupportedQuery("FORMAT".to_owned()));
    }
    if !query.pipe_operators.is_empty() {
        return Err(ParserError::UnsupportedQuery("pipe operator".to_owned()));
    }

    Ok(())
}

fn reject_select_extensions(select: &Select) -> Result<(), ParserError> {
    if !select.optimizer_hints.is_empty() {
        return Err(ParserError::UnsupportedQuery("optimizer hints".to_owned()));
    }
    if select.distinct.is_some() {
        return Err(ParserError::UnsupportedQuery("DISTINCT".to_owned()));
    }
    if select.select_modifiers.is_some() {
        return Err(ParserError::UnsupportedQuery("SELECT modifiers".to_owned()));
    }
    if select.top.is_some() {
        return Err(ParserError::UnsupportedQuery("TOP".to_owned()));
    }
    if select.exclude.is_some() {
        return Err(ParserError::UnsupportedQuery("EXCLUDE".to_owned()));
    }
    if select.into.is_some() {
        return Err(ParserError::UnsupportedQuery("INTO".to_owned()));
    }
    if !select.lateral_views.is_empty() {
        return Err(ParserError::UnsupportedQuery("LATERAL VIEW".to_owned()));
    }
    if select.prewhere.is_some() {
        return Err(ParserError::UnsupportedQuery("PREWHERE".to_owned()));
    }
    if !select.connect_by.is_empty() {
        return Err(ParserError::UnsupportedQuery("CONNECT BY".to_owned()));
    }

    match &select.group_by {
        GroupByExpr::All(_) => return Err(ParserError::UnsupportedQuery("GROUP BY".to_owned())),
        GroupByExpr::Expressions(expressions, modifiers)
            if !expressions.is_empty() || !modifiers.is_empty() =>
        {
            return Err(ParserError::UnsupportedQuery("GROUP BY".to_owned()));
        }
        GroupByExpr::Expressions(_, _) => {}
    }

    if !select.cluster_by.is_empty() {
        return Err(ParserError::UnsupportedQuery("CLUSTER BY".to_owned()));
    }
    if !select.distribute_by.is_empty() {
        return Err(ParserError::UnsupportedQuery("DISTRIBUTE BY".to_owned()));
    }
    if !select.sort_by.is_empty() {
        return Err(ParserError::UnsupportedQuery("SORT BY".to_owned()));
    }
    if select.having.is_some() {
        return Err(ParserError::UnsupportedQuery("HAVING".to_owned()));
    }
    if !select.named_window.is_empty() {
        return Err(ParserError::UnsupportedQuery("WINDOW".to_owned()));
    }
    if select.value_table_mode.is_some() {
        return Err(ParserError::UnsupportedQuery("value table mode".to_owned()));
    }

    Ok(())
}

fn lower_projection(item: &SelectItem) -> Result<String, ParserError> {
    let SelectItem::UnnamedExpr(expression) = item else {
        return Err(ParserError::UnsupportedProjection(item.to_string()));
    };

    validate_expression(expression)?;
    Ok(expression.to_string())
}

fn validate_expression(expression: &Expr) -> Result<(), ParserError> {
    match expression {
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => Ok(()),
        Expr::Value(value) => validate_value(&value.value, expression),
        Expr::Function(function) => {
            let rendered = function.to_string();
            let name = function.name.to_string();
            if !rendered.starts_with(&format!("{name}(")) || !rendered.ends_with(')') {
                return Err(ParserError::UnsupportedExpression(rendered));
            }
            Ok(())
        }
        Expr::Nested(inner) => validate_expression(inner),
        Expr::BinaryOp { left, op, right } if is_supported_binary_operator(op) => {
            validate_expression(left)?;
            validate_expression(right)
        }
        _ => Err(ParserError::UnsupportedExpression(expression.to_string())),
    }
}

fn validate_value(value: &Value, expression: &Expr) -> Result<(), ParserError> {
    match value {
        Value::Number(number, _) if number.chars().all(|character| character.is_ascii_digit()) => {
            Ok(())
        }
        Value::SingleQuotedString(_) | Value::DoubleQuotedString(_) => Ok(()),
        _ => Err(ParserError::UnsupportedExpression(expression.to_string())),
    }
}

fn is_supported_binary_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::LtEq
            | BinaryOperator::Gt
            | BinaryOperator::GtEq
            | BinaryOperator::And
            | BinaryOperator::Or
    )
}

fn lower_conditions(expression: &Expr, out: &mut Vec<ConditionIR>) -> Result<(), ParserError> {
    match expression {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And | BinaryOperator::Or,
            right,
        } => {
            lower_conditions(left, out)?;
            lower_conditions(right, out)
        }
        Expr::BinaryOp { left, op, right } => {
            let operator = comparison_operator(op)
                .ok_or_else(|| ParserError::UnsupportedExpression(expression.to_string()))?;
            let left = lower_atom(left)?;
            let right = lower_atom(right)?;
            out.push(ConditionIR::comparison(left, operator, right));
            Ok(())
        }
        _ => {
            let left = lower_atom(expression)?;
            out.push(ConditionIR::boolean(left));
            Ok(())
        }
    }
}

fn comparison_operator(operator: &BinaryOperator) -> Option<ConditionOperator> {
    match operator {
        BinaryOperator::Eq => Some(ConditionOperator::Equal),
        BinaryOperator::NotEq => Some(ConditionOperator::NotEqual),
        BinaryOperator::Lt => Some(ConditionOperator::LessThan),
        BinaryOperator::LtEq => Some(ConditionOperator::LessThanOrEqual),
        BinaryOperator::Gt => Some(ConditionOperator::GreaterThan),
        BinaryOperator::GtEq => Some(ConditionOperator::GreaterThanOrEqual),
        _ => None,
    }
}

fn lower_atom(expression: &Expr) -> Result<String, ParserError> {
    match expression {
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => Ok(expression.to_string()),
        Expr::Value(value) => {
            validate_value(&value.value, expression)?;
            Ok(expression.to_string())
        }
        Expr::Function(function) => {
            validate_expression(expression)?;
            Ok(format!("{}()", function.name))
        }
        _ => Err(ParserError::UnsupportedExpression(expression.to_string())),
    }
}

fn lower_join(join: &Join, natural: bool) -> Result<JoinIR, ParserError> {
    if join.global {
        return Err(ParserError::UnsupportedJoin("GLOBAL JOIN".to_owned()));
    }

    let (kind, constraint) = match &join.join_operator {
        JoinOperator::Join(constraint) | JoinOperator::Inner(constraint) => {
            (JoinKind::Inner, constraint)
        }
        JoinOperator::Left(constraint) | JoinOperator::LeftOuter(constraint) => {
            (JoinKind::Left, constraint)
        }
        JoinOperator::Right(constraint) | JoinOperator::RightOuter(constraint) => {
            (JoinKind::Right, constraint)
        }
        JoinOperator::FullOuter(constraint) => (JoinKind::Full, constraint),
        JoinOperator::CrossJoin(constraint) => (JoinKind::Cross, constraint),
        unsupported => return Err(ParserError::UnsupportedJoin(format!("{unsupported:?}"))),
    };

    let kind = if natural { kind.natural()? } else { kind };
    let table = plain_table_name(&join.relation, true)?;

    let JoinConstraint::On(expression) = constraint else {
        return Err(ParserError::UnsupportedJoinConstraint(format!(
            "{constraint:?}"
        )));
    };

    let mut conditions = Vec::new();
    lower_conditions(expression, &mut conditions)?;

    Ok(JoinIR {
        kind,
        table,
        conditions,
    })
}

fn plain_table_name(table: &TableFactor, joined: bool) -> Result<String, ParserError> {
    let TableFactor::Table { name, .. } = table else {
        return Err(ParserError::UnsupportedTable(table.to_string()));
    };

    if table.to_string() != name.to_string() {
        return Err(ParserError::UnsupportedTable(table.to_string()));
    }

    let rendered = name.to_string();
    if joined {
        rendered
            .split('.')
            .next()
            .map(str::to_owned)
            .ok_or(ParserError::UnsupportedTable(rendered))
    } else {
        Ok(rendered)
    }
}

#[derive(Debug)]
struct WordToken {
    start: usize,
    end: usize,
    upper: String,
}

fn normalize_natural_joins(sql: &str) -> Result<(String, Vec<bool>), ParserError> {
    let mut characters: Vec<char> = sql.chars().collect();
    let words = scan_words(&characters)?;

    let mut natural_ranges = Vec::new();
    let mut natural_flags = Vec::new();

    for (index, word) in words.iter().enumerate() {
        if word.upper != "JOIN" {
            continue;
        }

        let natural_index = natural_modifier_index(&words, index);
        natural_flags.push(natural_index.is_some());

        if let Some(natural_index) = natural_index {
            let natural = &words[natural_index];
            natural_ranges.push((natural.start, natural.end));
        }
    }

    for (start, end) in natural_ranges {
        for character in &mut characters[start..end] {
            *character = ' ';
        }
    }

    Ok((characters.into_iter().collect(), natural_flags))
}

fn natural_modifier_index(words: &[WordToken], join_index: usize) -> Option<usize> {
    if join_index == 0 {
        return None;
    }

    let mut index = join_index - 1;
    while matches!(
        words[index].upper.as_str(),
        "LEFT" | "RIGHT" | "FULL" | "INNER" | "OUTER"
    ) {
        if index == 0 {
            return None;
        }
        index -= 1;
    }

    if words[index].upper != "NATURAL" {
        return None;
    }

    if index > 0 && words[index - 1].upper == "FROM" {
        return None;
    }

    Some(index)
}

fn scan_words(characters: &[char]) -> Result<Vec<WordToken>, ParserError> {
    let mut words = Vec::new();
    let mut index = 0;

    while index < characters.len() {
        match characters[index] {
            '\'' | '"' => {
                index = skip_quoted(characters, index);
            }
            '-' if characters.get(index + 1) == Some(&'-') => {
                index += 2;
                while index < characters.len() && characters[index] != '\n' {
                    index += 1;
                }
            }
            '/' if characters.get(index + 1) == Some(&'*') => {
                return Err(ParserError::UnsupportedSyntax("block comments".to_owned()));
            }
            character if character.is_ascii_alphabetic() || character == '_' => {
                let start = index;
                index += 1;
                while index < characters.len()
                    && (characters[index].is_ascii_alphanumeric() || characters[index] == '_')
                {
                    index += 1;
                }
                let value: String = characters[start..index].iter().collect();
                words.push(WordToken {
                    start,
                    end: index,
                    upper: value.to_ascii_uppercase(),
                });
            }
            _ => index += 1,
        }
    }

    Ok(words)
}

fn skip_quoted(characters: &[char], start: usize) -> usize {
    let quote = characters[start];
    let mut index = start + 1;

    while index < characters.len() {
        if characters[index] == quote {
            if characters.get(index + 1) == Some(&quote) {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += 1;
    }

    index
}
