//! Generic JSON → Subsonic-XML walker. The Subsonic XML dialect maps
//! cleanly from JSON: scalar fields become attributes, nested objects
//! become child elements, arrays repeat the element per item, and the
//! reserved `"value"` key becomes text content (used by `<genre>`).

pub(crate) fn write_element(out: &mut String, name: &str, value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            out.push('<');
            out.push_str(name);

            let mut text: Option<String> = None;
            let mut children: Vec<(&String, &serde_json::Value)> = Vec::new();
            for (key, val) in map {
                match val {
                    serde_json::Value::Null => {}
                    serde_json::Value::String(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::Bool(_) => {
                        if key == "value" {
                            text = Some(scalar_str(val));
                        } else {
                            out.push(' ');
                            out.push_str(key);
                            out.push_str("=\"");
                            push_escaped(out, &scalar_str(val));
                            out.push('"');
                        }
                    }
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        children.push((key, val));
                    }
                }
            }

            if text.is_none() && children.is_empty() {
                out.push_str("/>");
                return;
            }
            out.push('>');
            if let Some(text) = text {
                push_escaped(out, &text);
            }
            for (key, val) in children {
                write_element(out, key, val);
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        serde_json::Value::Array(items) => {
            for item in items {
                write_element(out, name, item);
            }
        }
        serde_json::Value::Null => {}
        scalar => {
            out.push('<');
            out.push_str(name);
            out.push('>');
            push_escaped(out, &scalar_str(scalar));
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
    }
}

fn scalar_str(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn push_escaped(out: &mut String, raw: &str) {
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xml_of(name: &str, value: &serde_json::Value) -> String {
        let mut out = String::new();
        write_element(&mut out, name, value);
        out
    }

    #[test]
    fn scalars_become_attributes_objects_become_children() {
        let xml = xml_of(
            "artists",
            &serde_json::json!({
                "ignoredArticles": "",
                "index": [
                    { "name": "S", "artist": [ { "id": "ar-1", "name": "Soda Stereo", "albumCount": 7 } ] }
                ]
            }),
        );
        // serde_json orders keys alphabetically; attribute order is stable
        assert_eq!(
            xml,
            "<artists ignoredArticles=\"\"><index name=\"S\">\
             <artist albumCount=\"7\" id=\"ar-1\" name=\"Soda Stereo\"/>\
             </index></artists>"
        );
    }

    #[test]
    fn value_key_becomes_text_node() {
        let xml = xml_of(
            "genre",
            &serde_json::json!({ "songCount": 12, "albumCount": 0, "value": "Rock Nacional" }),
        );
        assert_eq!(
            xml,
            "<genre albumCount=\"0\" songCount=\"12\">Rock Nacional</genre>"
        );
    }

    #[test]
    fn escaping_in_attributes_and_text() {
        let xml = xml_of(
            "song",
            &serde_json::json!({ "title": "M\u{f6}tley Cr\u{fc}e & Friends <live>" }),
        );
        assert_eq!(
            xml,
            "<song title=\"M\u{f6}tley Cr\u{fc}e &amp; Friends &lt;live&gt;\"/>"
        );
    }

    #[test]
    fn nulls_are_skipped_arrays_repeat() {
        let xml = xml_of(
            "starred2",
            &serde_json::json!({ "song": [ {"id": "tr-1"}, {"id": "tr-2"} ], "gone": null }),
        );
        assert_eq!(xml, "<starred2><song id=\"tr-1\"/><song id=\"tr-2\"/></starred2>");
    }
}
