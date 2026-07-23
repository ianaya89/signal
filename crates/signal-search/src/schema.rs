#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Int,
    Bool,
    /// Value like `5m` / `90s` normalized to milliseconds.
    Duration,
    /// Relative date: `last-week`, `last-month`, or `YYYY-MM-DD`.
    Date,
}

#[derive(Debug)]
pub struct FieldDef {
    pub name: &'static str,
    /// SQL expression the field compiles to (against aliases t/ar/al).
    pub column: &'static str,
    pub kind: FieldType,
}

pub const FIELDS: &[FieldDef] = &[
    FieldDef {
        name: "artist",
        column: "ar.name",
        kind: FieldType::Text,
    },
    FieldDef {
        name: "album",
        column: "al.name",
        kind: FieldType::Text,
    },
    FieldDef {
        name: "title",
        column: "t.title",
        kind: FieldType::Text,
    },
    FieldDef {
        name: "genre",
        column: "GENRE",
        kind: FieldType::Text,
    }, // EXISTS subquery
    FieldDef {
        name: "year",
        column: "t.year",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "rating",
        column: "t.rating",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "codec",
        column: "t.codec",
        kind: FieldType::Text,
    },
    FieldDef {
        name: "bitdepth",
        column: "t.bit_depth",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "samplerate",
        column: "t.sample_rate_hz",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "bitrate",
        column: "t.bitrate_kbps",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "channels",
        column: "t.channels",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "duration",
        column: "t.duration_ms",
        kind: FieldType::Duration,
    },
    FieldDef {
        name: "playcount",
        column: "t.play_count",
        kind: FieldType::Int,
    },
    FieldDef {
        name: "favorite",
        column: "t.favorite",
        kind: FieldType::Bool,
    },
    FieldDef {
        name: "added",
        column: "t.added_at",
        kind: FieldType::Date,
    },
];

/// Case-insensitive lookup; also accepts camelCase aliases from the docs
/// (sampleRate, bitDepth, playCount).
#[must_use]
pub fn field(name: &str) -> Option<&'static FieldDef> {
    let lower = name.to_ascii_lowercase();
    FIELDS.iter().find(|f| f.name == lower)
}
