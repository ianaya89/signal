//! Smart-playlist rule compiler: the JSON format from docs/03 §4 into a
//! parameterized WHERE clause. Field and order-by names go through strict
//! whitelists — rule values never reach the SQL string.

use serde::{Deserialize, Serialize};

// field names stay snake_case: that's the stored-rules format (docs/03 §4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRules {
    #[serde(default = "default_match")]
    pub r#match: MatchMode,
    pub conditions: Vec<SmartCondition>,
    #[serde(default)]
    pub order_by: Option<String>,
    #[serde(default)]
    pub order_dir: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

fn default_match() -> MatchMode {
    MatchMode::All
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    All,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCondition {
    pub field: String,
    pub op: String,
    #[serde(default)]
    pub value: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum SmartError {
    #[error("invalid rules json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown field: {0}")]
    UnknownField(String),
    #[error("unknown operator: {0}")]
    UnknownOp(String),
    #[error("bad value for {field}: {value}")]
    BadValue { field: String, value: String },
}

pub enum Bind {
    Text(String),
    Int(i64),
    Real(f64),
}

/// (field name, SQL column, `is_text`). GENRE compiles to an EXISTS subquery.
const FIELDS: &[(&str, &str, bool)] = &[
    ("title", "t.title", true),
    ("artist_name", "ar.name", true),
    ("album_name", "al.name", true),
    ("genre", "GENRE", true),
    ("year", "t.year", false),
    ("rating", "t.rating", false),
    ("favorite", "t.favorite", false),
    ("play_count", "t.play_count", false),
    ("skip_count", "t.skip_count", false),
    ("added_at", "t.added_at", true),
    ("last_played_at", "t.last_played_at", true),
    ("codec", "t.codec", true),
    ("bit_depth", "t.bit_depth", false),
    ("sample_rate_hz", "t.sample_rate_hz", false),
    ("channels", "t.channels", false),
    ("duration_ms", "t.duration_ms", false),
];

const ORDER_FIELDS: &[&str] = &[
    "added_at",
    "last_played_at",
    "play_count",
    "year",
    "rating",
    "sample_rate_hz",
    "bit_depth",
    "duration_ms",
    "title",
];

/// Compiles rules to (`where_clause`, binds, `order_limit_clause`).
pub fn compile(rules_json: &str) -> Result<(String, Vec<Bind>, String), SmartError> {
    let rules: SmartRules = serde_json::from_str(rules_json)?;

    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    for cond in &rules.conditions {
        let (_, column, is_text) = FIELDS
            .iter()
            .find(|(name, _, _)| *name == cond.field)
            .ok_or_else(|| SmartError::UnknownField(cond.field.clone()))?;
        clauses.push(compile_condition(cond, column, *is_text, &mut binds)?);
    }

    let joiner = match rules.r#match {
        MatchMode::All => " AND ",
        MatchMode::Any => " OR ",
    };
    let where_clause = if clauses.is_empty() {
        "1=1".to_owned()
    } else {
        format!("({})", clauses.join(joiner))
    };

    let mut tail = String::new();
    if let Some(order_by) = &rules.order_by {
        let field = ORDER_FIELDS
            .iter()
            .find(|f| **f == order_by.as_str())
            .ok_or_else(|| SmartError::UnknownField(order_by.clone()))?;
        let dir = match rules.order_dir.as_deref() {
            Some("desc") => "DESC",
            _ => "ASC",
        };
        let column = FIELDS
            .iter()
            .find(|(name, _, _)| name == field)
            .map_or("t.added_at", |(_, col, _)| *col);
        tail = format!(" ORDER BY {column} {dir}");
    }
    if let Some(limit) = rules.limit {
        use std::fmt::Write as _;
        let _ = write!(tail, " LIMIT {limit}");
    }

    Ok((where_clause, binds, tail))
}

fn compile_condition(
    cond: &SmartCondition,
    column: &str,
    is_text: bool,
    binds: &mut Vec<Bind>,
) -> Result<String, SmartError> {
    let bad = || SmartError::BadValue {
        field: cond.field.clone(),
        value: cond.value.to_string(),
    };

    let mut comparison = |op: &str| -> Result<String, SmartError> {
        let bind = json_to_bind(&cond.value).ok_or_else(bad)?;
        binds.push(bind);
        if column == "GENRE" {
            return Ok(format!(
                "EXISTS (SELECT 1 FROM track_genres tg JOIN genres g ON g.id = tg.genre_id \
                 WHERE tg.track_id = t.id AND g.name {op} ? COLLATE NOCASE)"
            ));
        }
        Ok(format!("{column} {op} ?"))
    };

    match cond.op.as_str() {
        "eq" => comparison("="),
        "neq" => comparison("<>"),
        "gt" => comparison(">"),
        "gte" => comparison(">="),
        "lt" => comparison("<"),
        "lte" => comparison("<="),
        "contains" | "not_contains" => {
            let text = cond.value.as_str().ok_or_else(bad)?;
            binds.push(Bind::Text(format!("%{text}%")));
            let neg = if cond.op == "not_contains" {
                "NOT "
            } else {
                ""
            };
            if column == "GENRE" {
                Ok(format!(
                    "{neg}EXISTS (SELECT 1 FROM track_genres tg JOIN genres g ON g.id = tg.genre_id \
                     WHERE tg.track_id = t.id AND g.name LIKE ? COLLATE NOCASE)"
                ))
            } else if is_text {
                Ok(format!("{neg}({column} LIKE ? COLLATE NOCASE)"))
            } else {
                Err(bad())
            }
        }
        "is_null" => Ok(format!("{column} IS NULL")),
        "is_not_null" => Ok(format!("{column} IS NOT NULL")),
        "within_days" => {
            let days = cond.value.as_u64().ok_or_else(bad)?;
            Ok(format!("date({column}) >= date('now', '-{days} days')"))
        }
        other => Err(SmartError::UnknownOp(other.to_owned())),
    }
}

fn json_to_bind(value: &serde_json::Value) -> Option<Bind> {
    match value {
        serde_json::Value::String(s) => Some(Bind::Text(s.clone())),
        serde_json::Value::Bool(b) => Some(Bind::Int(i64::from(*b))),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(Bind::Int)
            .or_else(|| n.as_f64().map(Bind::Real)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn never_played_compiles() {
        let (where_c, binds, tail) = compile(
            r#"{"match":"all","conditions":[{"field":"play_count","op":"eq","value":0}],"order_by":"added_at","order_dir":"desc","limit":null}"#,
        )
        .unwrap();
        assert_eq!(where_c, "(t.play_count = ?)");
        assert_eq!(binds.len(), 1);
        assert!(tail.contains("ORDER BY t.added_at DESC"));
    }

    #[test]
    fn genre_and_year() {
        let (where_c, binds, _) = compile(
            r#"{"match":"all","conditions":[
                {"field":"genre","op":"contains","value":"Jazz"},
                {"field":"year","op":"gt","value":1990}
            ],"order_by":"year","order_dir":"asc"}"#,
        )
        .unwrap();
        assert!(where_c.contains("EXISTS (SELECT 1 FROM track_genres"));
        assert!(where_c.contains("t.year > ?"));
        assert!(where_c.contains(" AND "));
        assert_eq!(binds.len(), 2);
    }

    #[test]
    fn within_days_inlines_validated_number() {
        let (where_c, binds, _) =
            compile(r#"{"conditions":[{"field":"added_at","op":"within_days","value":30}]}"#)
                .unwrap();
        assert!(where_c.contains("date('now', '-30 days')"));
        assert!(binds.is_empty());
    }

    #[test]
    fn unknown_field_rejected() {
        assert!(compile(r#"{"conditions":[{"field":"evil; DROP","op":"eq","value":1}]}"#).is_err());
    }

    #[test]
    fn any_uses_or() {
        let (where_c, _, _) = compile(
            r#"{"match":"any","conditions":[
                {"field":"rating","op":"gte","value":4},
                {"field":"favorite","op":"eq","value":true}
            ]}"#,
        )
        .unwrap();
        assert!(where_c.contains(" OR "));
    }
}
