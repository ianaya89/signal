//! Search: query-language parser and compilation to parameterized SQL
//! mixing FTS5 MATCH with structured WHERE predicates.
//!
//! Grammar: bare terms become full-text search; `field:value`, `field>n`,
//! `field>=n`, etc. become column predicates. Fields and operators are
//! validated against [`schema::FIELDS`].

#![allow(clippy::missing_errors_doc)]

mod compile;
mod parser;
mod schema;

pub use compile::{compile, CompiledQuery};
pub use parser::{parse, Condition, Op, QueryAst, SearchError};
pub use schema::{FieldDef, FieldType, FIELDS};

use signal_core::Track;
use signal_db::DbPool;

/// Parse + compile + execute in one call. Returns matching tracks.
pub async fn search(db: &DbPool, query: &str, limit: u32) -> Result<Vec<Track>, SearchError> {
    let ast = parse(query)?;
    let compiled = compile(&ast, limit);
    compiled
        .fetch(db)
        .await
        .map_err(|e| SearchError::Execution(e.to_string()))
}
