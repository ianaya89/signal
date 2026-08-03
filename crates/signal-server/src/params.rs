//! Query-string params with repeated-key support (`star?id=tr-1&id=tr-2`),
//! which rules out axum's `Query<HashMap>` extractor.

use crate::envelope::ApiError;

pub(crate) struct Params(Vec<(String, String)>);

impl Params {
    pub fn parse(query: &str) -> Self {
        Self(serde_urlencoded::from_str::<Vec<(String, String)>>(query).unwrap_or_default())
    }

    /// Query string + formPost body; body pairs append so either source works.
    pub fn parse_merged(query: &str, form_body: &str) -> Self {
        let mut pairs = Self::parse(query).0;
        pairs.extend(Self::parse(form_body).0);
        Self(pairs)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    pub fn require(&self, key: &str) -> Result<&str, ApiError> {
        self.get(key).ok_or_else(|| ApiError::missing_param(key))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn repeated_keys_survive() {
        let p = Params::parse("id=tr-1&id=tr-2&u=me&size=50");
        assert_eq!(p.get_all("id"), vec!["tr-1", "tr-2"]);
        assert_eq!(p.get("u"), Some("me"));
        assert_eq!(p.get_u32("size"), Some(50));
        assert!(p.require("missing").is_err());
    }

    #[test]
    fn percent_decoding_applies() {
        let p = Params::parse("query=so%20much%20%26%20more");
        assert_eq!(p.get("query"), Some("so much & more"));
    }
}
