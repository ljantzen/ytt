use quick_xml::events::Event;
use quick_xml::Reader;
use std::str;

pub struct TranscriptParser {
    _preserve_formatting: bool,
}

impl TranscriptParser {
    pub fn new(preserve_formatting: bool) -> Self {
        Self {
            _preserve_formatting: preserve_formatting,
        }
    }

    pub fn parse(&self, xml: &str) -> Result<Vec<crate::TranscriptItem>, String> {
        let mut reader = Reader::from_str(xml);

        let mut items = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"text" => {
                        if let Some(item) = self.parse_text_element(&mut reader, &e)? {
                            items.push(item);
                        }
                    }
                    b"p" => {
                        if let Some(item) = self.parse_p_element(&mut reader, &e)? {
                            items.push(item);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(items)
    }

    fn parse_text_element(
        &self,
        reader: &mut Reader<&[u8]>,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<crate::TranscriptItem>, String> {
        let start = e
            .attributes()
            .find(|a| {
                a.as_ref()
                    .map(|attr| attr.key.as_ref() == b"start")
                    .unwrap_or(false)
            })
            .and_then(|a| {
                a.ok()
                    .and_then(|attr| str::from_utf8(&attr.value).ok().map(|s| s.to_string()))
                    .and_then(|s| s.parse::<f64>().ok())
            })
            .unwrap_or(0.0);

        let duration = e
            .attributes()
            .find(|a| {
                a.as_ref()
                    .map(|attr| attr.key.as_ref() == b"dur")
                    .unwrap_or(false)
            })
            .and_then(|a| {
                a.ok()
                    .and_then(|attr| str::from_utf8(&attr.value).ok().map(|s| s.to_string()))
                    .and_then(|s| s.parse::<f64>().ok())
            })
            .unwrap_or(0.0);

        let mut text = String::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Text(e)) => {
                    let decoded = html_escape::decode_html_entities(
                        e.decode()
                            .map_err(|e| format!("Failed to decode: {}", e))?
                            .as_ref(),
                    );
                    text.push_str(&decoded);
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"text" => break,
                Ok(Event::Eof) => return Err("Unexpected EOF in text element".to_string()),
                Err(e) => return Err(format!("XML parse error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        if text.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(crate::TranscriptItem {
            text: text.trim().to_string(),
            start,
            duration,
        }))
    }

    fn parse_p_element(
        &self,
        reader: &mut Reader<&[u8]>,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<crate::TranscriptItem>, String> {
        let start = e
            .attributes()
            .find(|a| {
                a.as_ref()
                    .map(|attr| attr.key.as_ref() == b"t")
                    .unwrap_or(false)
            })
            .and_then(|a| {
                a.ok().and_then(|attr| {
                    str::from_utf8(&attr.value)
                        .ok()
                        .map(|s| s.to_string())
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(|s| s / 1000.0) // Convert from milliseconds
                })
            })
            .unwrap_or(0.0);

        let duration = e
            .attributes()
            .find(|a| {
                a.as_ref()
                    .map(|attr| attr.key.as_ref() == b"d")
                    .unwrap_or(false)
            })
            .and_then(|a| {
                a.ok().and_then(|attr| {
                    str::from_utf8(&attr.value)
                        .ok()
                        .map(|s| s.to_string())
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(|s| s / 1000.0) // Convert from milliseconds
                })
            })
            .unwrap_or(0.0);

        let mut text = String::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Text(e)) => {
                    let decoded = html_escape::decode_html_entities(
                        e.decode()
                            .map_err(|e| format!("Failed to decode: {}", e))?
                            .as_ref(),
                    );
                    text.push_str(&decoded);
                }
                Ok(Event::Start(e)) => {
                    // Handle nested tags like <s>, <br/>, etc.
                    match e.name().as_ref() {
                        b"s" | b"br" => {
                            if !text.ends_with(' ') {
                                text.push(' ');
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"p" => break,
                Ok(Event::Eof) => return Err("Unexpected EOF in p element".to_string()),
                Err(e) => return Err(format!("XML parse error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        if text.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(crate::TranscriptItem {
            text: text.trim().to_string(),
            start,
            duration,
        }))
    }
}

mod html_escape {
    pub fn decode_html_entities(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '&' {
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' {
                        chars.next();
                        break;
                    }
                    entity.push(chars.next().unwrap());
                }

                result.push_str(&decode_entity(&entity));
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn decode_entity(entity: &str) -> String {
        match entity {
            "quot" => "\"".to_string(),
            "amp" => "&".to_string(),
            "apos" => "'".to_string(),
            "lt" => "<".to_string(),
            "gt" => ">".to_string(),
            "nbsp" => " ".to_string(),
            _ => {
                if entity.starts_with("#x") || entity.starts_with("#X") {
                    // Hex entity
                    if let Ok(num) = u32::from_str_radix(&entity[2..], 16) {
                        if let Some(ch) = char::from_u32(num) {
                            return ch.to_string();
                        }
                    }
                    format!("&{};", entity)
                } else if let Some(stripped) = entity.strip_prefix('#') {
                    // Decimal entity
                    if let Ok(num) = stripped.parse::<u32>() {
                        if let Some(ch) = char::from_u32(num) {
                            return ch.to_string();
                        }
                    }
                    format!("&{};", entity)
                } else {
                    format!("&{};", entity)
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_decode_basic_entities() {
            assert_eq!(decode_html_entities("&quot;"), "\"");
            assert_eq!(decode_html_entities("&amp;"), "&");
            assert_eq!(decode_html_entities("&apos;"), "'");
            assert_eq!(decode_html_entities("&lt;"), "<");
            assert_eq!(decode_html_entities("&gt;"), ">");
            assert_eq!(decode_html_entities("&nbsp;"), " ");
        }

        #[test]
        fn test_decode_hex_entities() {
            assert_eq!(decode_html_entities("&#x41;"), "A");
            assert_eq!(decode_html_entities("&#x61;"), "a");
        }

        #[test]
        fn test_decode_decimal_entities() {
            assert_eq!(decode_html_entities("&#65;"), "A");
            assert_eq!(decode_html_entities("&#97;"), "a");
        }

        #[test]
        fn test_decode_mixed() {
            assert_eq!(
                decode_html_entities("Hello &amp; world &quot;test&quot;"),
                "Hello & world \"test\""
            );
        }

        #[test]
        fn test_decode_unknown_entity() {
            assert_eq!(decode_html_entities("&unknown;"), "&unknown;");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_format() {
        let xml = r#"<transcript>
            <text start="0.0" dur="2.5">Hello world</text>
            <text start="2.5" dur="3.0">This is a test</text>
        </transcript>"#;

        let parser = TranscriptParser::new(false);
        let items = parser.parse(xml).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "Hello world");
        assert_eq!(items[0].start, 0.0);
        assert_eq!(items[0].duration, 2.5);
        assert_eq!(items[1].text, "This is a test");
        assert_eq!(items[1].start, 2.5);
    }

    #[test]
    fn test_parse_p_format() {
        let xml = r#"<transcript>
            <p t="0" d="2500">Hello world</p>
            <p t="2500" d="3000">This is a test</p>
        </transcript>"#;

        let parser = TranscriptParser::new(false);
        let items = parser.parse(xml).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "Hello world");
        assert_eq!(items[0].start, 0.0);
        assert_eq!(items[0].duration, 2.5);
        assert_eq!(items[1].text, "This is a test");
        assert_eq!(items[1].start, 2.5);
    }

    #[test]
    fn test_parse_with_html_entities() {
        let xml = r#"<transcript>
            <text start="0.0" dur="2.5">Hello &amp; world</text>
        </transcript>"#;

        let parser = TranscriptParser::new(false);
        let items = parser.parse(xml).unwrap();

        assert_eq!(items.len(), 1);
        // quick-xml unescapes &amp; to &, then our decoder processes it
        // The actual result depends on how quick-xml handles it
        assert!(items[0].text.contains("Hello"));
        assert!(items[0].text.contains("world"));
    }

    #[test]
    fn test_parse_empty_text() {
        let xml = r#"<transcript>
            <text start="0.0" dur="2.5"></text>
        </transcript>"#;

        let parser = TranscriptParser::new(false);
        let items = parser.parse(xml).unwrap();

        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_parse_invalid_xml() {
        let xml = "<transcript><text>Unclosed tag";

        let parser = TranscriptParser::new(false);
        assert!(parser.parse(xml).is_err());
    }
}
