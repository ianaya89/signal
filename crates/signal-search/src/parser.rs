use crate::schema::{self, FieldDef, FieldType};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SearchError {
    #[error("unknown field: {0}")]
    UnknownField(String),
    #[error("invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
    #[error("operator {op} not valid for {field}")]
    InvalidOp { op: String, field: String },
    #[error("query failed: {0}")]
    Execution(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Text(String),
    Int(i64),
    Bool(bool),
}

#[derive(Debug)]
pub struct Condition {
    pub field: &'static FieldDef,
    pub op: Op,
    pub value: Value,
}

#[derive(Debug, Default)]
pub struct QueryAst {
    /// Bare terms, joined for FTS5 MATCH. Empty = no full-text component.
    pub fts_terms: Vec<String>,
    pub conditions: Vec<Condition>,
}

/// Splits on whitespace; a token is either `field<op>value` or a bare term.
/// Quoted values (`artist:"pink floyd"`) keep spaces.
pub fn parse(input: &str) -> Result<QueryAst, SearchError> {
    let mut ast = QueryAst::default();

    for token in tokenize(input) {
        match split_condition(&token) {
            Some((field_raw, op, value_raw)) => {
                let Some(field) = schema::field(field_raw) else {
                    // unknown field: treat whole token as full-text, not error —
                    // "AC:DC" should search text, not fail
                    ast.fts_terms.push(token);
                    continue;
                };
                ast.conditions.push(condition(field, op, value_raw)?);
            }
            None => ast.fts_terms.push(token),
        }
    }

    Ok(ast)
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// `field:value` / `field>value` / `field>=value` → (field, op, value).
fn split_condition(token: &str) -> Option<(&str, Op, &str)> {
    for (needle, op) in [
        (">=", Op::Gte),
        ("<=", Op::Lte),
        (">", Op::Gt),
        ("<", Op::Lt),
        (":", Op::Eq),
    ] {
        if let Some(idx) = token.find(needle) {
            let (field, rest) = token.split_at(idx);
            let value = &rest[needle.len()..];
            if !field.is_empty() && !value.is_empty() {
                return Some((field, op, value));
            }
        }
    }
    None
}

fn condition(field: &'static FieldDef, op: Op, raw: &str) -> Result<Condition, SearchError> {
    let invalid = || SearchError::InvalidValue {
        field: field.name.to_owned(),
        value: raw.to_owned(),
    };

    let value = match field.kind {
        FieldType::Text => {
            if op != Op::Eq {
                return Err(SearchError::InvalidOp {
                    op: format!("{op:?}"),
                    field: field.name.to_owned(),
                });
            }
            Value::Text(raw.to_owned())
        }
        FieldType::Int => Value::Int(parse_int(raw).ok_or_else(invalid)?),
        FieldType::Bool => Value::Bool(match raw {
            "true" | "yes" | "1" => true,
            "false" | "no" | "0" => false,
            _ => return Err(invalid()),
        }),
        FieldType::Duration => Value::Int(parse_duration_ms(raw).ok_or_else(invalid)?),
        FieldType::Date => Value::Text(parse_date(raw).ok_or_else(invalid)?),
    };

    Ok(Condition { field, op, value })
}

/// Accepts `2000`, `44.1k`, `96k` (k = *1000, for sample rates).
fn parse_int(raw: &str) -> Option<i64> {
    if let Some(k) = raw.strip_suffix(['k', 'K']) {
        #[allow(clippy::cast_possible_truncation)]
        return k.parse::<f64>().ok().map(|v| (v * 1000.0).round() as i64);
    }
    raw.parse().ok()
}

/// `5m` → 300000, `90s` → 90000, bare number = seconds.
fn parse_duration_ms(raw: &str) -> Option<i64> {
    if let Some(m) = raw.strip_suffix('m') {
        return m.parse::<i64>().ok().map(|v| v * 60_000);
    }
    if let Some(s) = raw.strip_suffix('s') {
        return s.parse::<i64>().ok().map(|v| v * 1000);
    }
    raw.parse::<i64>().ok().map(|v| v * 1000)
}

/// Relative keywords compile to `SQLite` `date()` expressions; absolute dates
/// pass through (ISO sorts lexically).
fn parse_date(raw: &str) -> Option<String> {
    match raw {
        "today" => Some("date('now')".to_owned()),
        "last-week" => Some("date('now', '-7 days')".to_owned()),
        "last-month" => Some("date('now', '-1 month')".to_owned()),
        "last-year" => Some("date('now', '-1 year')".to_owned()),
        _ if raw.len() == 10 && raw.as_bytes()[4] == b'-' => Some(format!("'{raw}'")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn bare_terms_go_to_fts() {
        let ast = parse("bocanada cerati").unwrap();
        assert_eq!(ast.fts_terms, vec!["bocanada", "cerati"]);
        assert!(ast.conditions.is_empty());
    }

    #[test]
    fn field_conditions() {
        let ast = parse("artist:cerati year>1998 rating>=4 codec:flac").unwrap();
        assert_eq!(ast.conditions.len(), 4);
        assert_eq!(ast.conditions[0].field.name, "artist");
        assert_eq!(ast.conditions[1].op, Op::Gt);
        assert_eq!(ast.conditions[1].value, Value::Int(1998));
        assert_eq!(ast.conditions[2].op, Op::Gte);
    }

    #[test]
    fn quoted_values_keep_spaces() {
        let ast = parse("album:\"the dark side\"").unwrap();
        assert_eq!(ast.conditions[0].value, Value::Text("the dark side".into()));
    }

    #[test]
    fn samplerate_k_suffix_and_camelcase() {
        let ast = parse("sampleRate>48k bitDepth:24").unwrap();
        assert_eq!(ast.conditions[0].value, Value::Int(48_000));
        assert_eq!(ast.conditions[1].value, Value::Int(24));
    }

    #[test]
    fn duration_units() {
        let ast = parse("duration>5m").unwrap();
        assert_eq!(ast.conditions[0].value, Value::Int(300_000));
    }

    #[test]
    fn date_keywords() {
        let ast = parse("added:last-week").unwrap();
        assert_eq!(
            ast.conditions[0].value,
            Value::Text("date('now', '-7 days')".into())
        );
    }

    #[test]
    fn unknown_field_falls_back_to_fts() {
        let ast = parse("AC:DC").unwrap();
        assert!(ast.conditions.is_empty());
        assert_eq!(ast.fts_terms, vec!["AC:DC"]);
    }

    #[test]
    fn text_field_rejects_range_op() {
        assert!(matches!(
            parse("artist>cerati"),
            Err(SearchError::InvalidOp { .. })
        ));
    }

    #[test]
    fn bad_int_value_errors() {
        assert!(matches!(
            parse("year:banana"),
            Err(SearchError::InvalidValue { .. })
        ));
    }
}
