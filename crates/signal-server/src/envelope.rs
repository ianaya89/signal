//! The `subsonic-response` envelope: status, protocol version, `OpenSubsonic`
//! identity fields, and the error-code table. Enveloped endpoints always
//! answer HTTP 200 (Subsonic convention); only binaries use real statuses.

use axum::http::header;
use axum::response::{IntoResponse, Response};

pub(crate) const API_VERSION: &str = "1.16.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Xml,
    Json,
}

impl Format {
    pub fn from_param(f: Option<&str>) -> Self {
        // jsonp deliberately unsupported; render() reports it as an error in
        // plain json rather than guessing a callback wrapper
        match f {
            Some("json" | "jsonp") => Self::Json,
            _ => Self::Xml,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    pub code: u32,
    pub message: String,
}

impl ApiError {
    pub fn generic(message: impl Into<String>) -> Self {
        Self {
            code: 0,
            message: message.into(),
        }
    }

    /// Database failures — logged, then reported as generic (code 0).
    pub fn db(err: impl std::fmt::Display) -> Self {
        tracing::warn!("opensubsonic db error: {err}");
        Self::generic("internal database error")
    }

    pub fn missing_param(name: &str) -> Self {
        Self {
            code: 10,
            message: format!("required parameter '{name}' is missing"),
        }
    }

    pub fn wrong_credentials() -> Self {
        Self {
            code: 40,
            message: "wrong username or password".into(),
        }
    }

    pub fn not_authorized(message: impl Into<String>) -> Self {
        Self {
            code: 50,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: 70,
            message: message.into(),
        }
    }
}

/// `Some((key, value))` payload lands inside the envelope under `key`;
/// `None` is a bare ok (ping).
pub(crate) type Payload = Option<(&'static str, serde_json::Value)>;
pub(crate) type HandlerResult = Result<Payload, ApiError>;

pub(crate) fn render(format: Format, server_version: &str, result: HandlerResult) -> Response {
    let mut envelope = serde_json::json!({
        "status": if result.is_ok() { "ok" } else { "failed" },
        "version": API_VERSION,
        "type": "signal",
        "serverVersion": server_version,
        "openSubsonic": true,
    });
    let obj = envelope
        .as_object_mut()
        .unwrap_or_else(|| unreachable!("envelope is an object literal"));
    match result {
        Ok(Some((key, value))) => {
            obj.insert(key.to_owned(), value);
        }
        Ok(None) => {}
        Err(err) => {
            obj.insert(
                "error".to_owned(),
                serde_json::json!({ "code": err.code, "message": err.message }),
            );
        }
    }

    match format {
        Format::Json => {
            let body = serde_json::json!({ "subsonic-response": envelope }).to_string();
            ([(header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        Format::Xml => {
            let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
            obj.insert(
                "xmlns".to_owned(),
                serde_json::Value::String("http://subsonic.org/restapi".to_owned()),
            );
            crate::xml::write_element(&mut out, "subsonic-response", &envelope);
            ([(header::CONTENT_TYPE, "text/xml; charset=utf-8")], out).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    async fn body_of(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn json_ok_with_payload() {
        let resp = render(
            Format::Json,
            "0.1.4",
            Ok(Some(("license", serde_json::json!({"valid": true})))),
        );
        let body = body_of(resp).await;
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let env = &v["subsonic-response"];
        assert_eq!(env["status"], "ok");
        assert_eq!(env["version"], "1.16.1");
        assert_eq!(env["openSubsonic"], true);
        assert_eq!(env["license"]["valid"], true);
    }

    #[tokio::test]
    async fn xml_failure_carries_error_code() {
        let resp = render(Format::Xml, "0.1.4", Err(ApiError::wrong_credentials()));
        let body = body_of(resp).await;
        assert!(body.contains("status=\"failed\""), "{body}");
        assert!(body.contains("<error code=\"40\""), "{body}");
        assert!(body.contains("xmlns=\"http://subsonic.org/restapi\""), "{body}");
    }
}
