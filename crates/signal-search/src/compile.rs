use std::fmt::Write as _;

use signal_core::Track;
use signal_db::DbPool;

use crate::parser::{Condition, Op, QueryAst, Value};

/// A ready-to-run SQL query with bind parameters.
pub struct CompiledQuery {
    pub sql: String,
    pub binds: Vec<Bind>,
}

#[derive(Debug, Clone)]
pub enum Bind {
    Text(String),
    Int(i64),
}

impl Op {
    fn sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

/// Builds one SELECT over tracks joined with artists/albums, with an FTS5
/// subquery filter when bare terms are present. Parameterized throughout —
/// user input never lands in the SQL string, except validated date
/// expressions produced by the parser itself.
#[must_use]
pub fn compile(ast: &QueryAst, limit: u32) -> CompiledQuery {
    let mut sql = String::from(
        "SELECT t.* FROM tracks t \
         JOIN artists ar ON ar.id = t.artist_id \
         LEFT JOIN albums al ON al.id = t.album_id WHERE 1=1",
    );
    let mut binds: Vec<Bind> = Vec::new();

    if !ast.fts_terms.is_empty() {
        // implicit AND of prefix-matched terms
        let match_expr = ast
            .fts_terms
            .iter()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");
        sql.push_str(" AND t.id IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?)");
        binds.push(Bind::Text(match_expr));
    }

    for cond in &ast.conditions {
        compile_condition(cond, &mut sql, &mut binds);
    }

    sql.push_str(" ORDER BY ar.name COLLATE NOCASE, al.name COLLATE NOCASE, t.disc_no, t.track_no");
    let _ = write!(sql, " LIMIT {limit}");

    CompiledQuery { sql, binds }
}

fn compile_condition(cond: &Condition, sql: &mut String, binds: &mut Vec<Bind>) {
    let column = cond.field.column;

    match (&cond.value, column) {
        // genre: EXISTS against the join table
        (Value::Text(text), "GENRE") => {
            sql.push_str(
                " AND EXISTS (SELECT 1 FROM track_genres tg \
                 JOIN genres g ON g.id = tg.genre_id \
                 WHERE tg.track_id = t.id AND g.name LIKE ? COLLATE NOCASE)",
            );
            binds.push(Bind::Text(format!("%{text}%")));
        }
        // date fields: parser emitted a SQLite date() expression or quoted literal
        (Value::Text(expr), _) if cond.field.name == "added" => {
            let _ = write!(
                sql,
                " AND date(t.added_at) {} {expr}",
                if cond.op == Op::Eq {
                    ">="
                } else {
                    cond.op.sql()
                }
            );
        }
        (Value::Text(text), _) => {
            let _ = write!(sql, " AND {column} LIKE ? COLLATE NOCASE");
            binds.push(Bind::Text(format!("%{text}%")));
        }
        (Value::Int(n), _) => {
            let _ = write!(sql, " AND {column} {} ?", cond.op.sql());
            binds.push(Bind::Int(*n));
        }
        (Value::Bool(b), _) => {
            let _ = write!(sql, " AND {column} = ?");
            binds.push(Bind::Int(i64::from(*b)));
        }
    }
}

impl CompiledQuery {
    pub async fn fetch(&self, db: &DbPool) -> sqlx::Result<Vec<Track>> {
        let mut query = sqlx::query(&self.sql);
        for bind in &self.binds {
            query = match bind {
                Bind::Text(s) => query.bind(s.clone()),
                Bind::Int(n) => query.bind(*n),
            };
        }
        let rows = query.fetch_all(db.inner()).await?;
        rows.iter().map(signal_db::track_from_row).collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::parser::parse;

    #[test]
    fn fts_plus_conditions() {
        let ast = parse("cerati codec:flac year>1998").unwrap();
        let q = compile(&ast, 50);
        assert!(q.sql.contains("tracks_fts MATCH ?"));
        assert!(q.sql.contains("t.codec LIKE ? COLLATE NOCASE"));
        assert!(q.sql.contains("t.year > ?"));
        assert!(q.sql.contains("LIMIT 50"));
        assert_eq!(q.binds.len(), 3);
    }

    #[test]
    fn genre_uses_exists_subquery() {
        let ast = parse("genre:jazz").unwrap();
        let q = compile(&ast, 10);
        assert!(q.sql.contains("EXISTS (SELECT 1 FROM track_genres"));
    }

    #[test]
    fn date_condition_inlines_expression_not_value() {
        let ast = parse("added:last-week").unwrap();
        let q = compile(&ast, 10);
        assert!(q.sql.contains("date(t.added_at) >= date('now', '-7 days')"));
        assert!(q.binds.is_empty());
    }

    #[test]
    fn fts_quotes_stripped_prefix_added() {
        let ast = parse("boca\"nada").unwrap();
        let q = compile(&ast, 10);
        match &q.binds[0] {
            Bind::Text(s) => assert_eq!(s, "\"bocanada\"*"),
            Bind::Int(_) => panic!("expected text bind"),
        }
    }
}
