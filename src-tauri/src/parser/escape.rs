/// Decodes a pg_dump COPY field value.
/// Returns `None` if the field is `\N` (SQL NULL), `Some(String)` otherwise.
pub fn decode_field(raw: &str) -> Option<String> {
    if raw == "\\N" {
        return None;
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000C}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('v') => out.push('\u{000B}'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_marker_returns_none() {
        assert_eq!(decode_field("\\N"), None);
    }

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(decode_field("hello world"), Some("hello world".into()));
    }

    #[test]
    fn empty_string_is_not_null() {
        assert_eq!(decode_field(""), Some(String::new()));
    }

    #[test]
    fn backslash_backslash_to_single_backslash() {
        assert_eq!(decode_field("a\\\\b"), Some("a\\b".into()));
    }

    #[test]
    fn tab_escape() {
        assert_eq!(decode_field("a\\tb"), Some("a\tb".into()));
    }

    #[test]
    fn newline_escape() {
        assert_eq!(decode_field("a\\nb"), Some("a\nb".into()));
    }

    #[test]
    fn carriage_return_escape() {
        assert_eq!(decode_field("a\\rb"), Some("a\rb".into()));
    }

    #[test]
    fn multiple_escapes_in_sequence() {
        assert_eq!(decode_field("a\\tb\\nc"), Some("a\tb\nc".into()));
    }

    #[test]
    fn backslash_then_n_inside_value_not_null() {
        // \N is only NULL when it's the entire field. "foo\Nbar" -> unknown escape, keep 'N'.
        assert_eq!(decode_field("foo\\Nbar"), Some("fooNbar".into()));
    }

    #[test]
    fn trailing_lone_backslash_kept() {
        assert_eq!(decode_field("foo\\"), Some("foo\\".into()));
    }
}
