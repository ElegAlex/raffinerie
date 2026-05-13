#[derive(Debug, PartialEq, Eq)]
pub struct CopyHeader {
    pub table: String,
    pub columns: Vec<String>,
}

/// Parse a COPY header line. Returns None if the line is not a COPY public.<table> header.
pub fn parse_copy_header(line: &str) -> Option<CopyHeader> {
    let line = line.trim();
    let rest = line.strip_prefix("COPY public.")?;
    // Extract table name: characters up to whitespace or '('
    let table_end = rest.find(|c: char| c.is_whitespace() || c == '(')?;
    let table = rest[..table_end].to_string();
    if table.is_empty() {
        return None;
    }
    let cols_start = line.find('(')?;
    let cols_end = line.find(')')?;
    if cols_end <= cols_start + 1 {
        return None;
    }
    let cols_raw = &line[cols_start + 1..cols_end];
    let columns: Vec<String> = cols_raw
        .split(',')
        .map(|c| c.trim().trim_matches('"').to_string())
        .filter(|c| !c.is_empty())
        .collect();
    if columns.is_empty() {
        return None;
    }
    Some(CopyHeader { table, columns })
}

use chrono::NaiveDate;

/// A row of decoded field values, indexable by column name.
#[derive(Debug)]
pub struct Row<'a> {
    columns: &'a [String],
    values: Vec<Option<String>>,
}

impl<'a> Row<'a> {
    pub fn parse(columns: &'a [String], line: &str) -> Result<Self, String> {
        let raw_fields: Vec<&str> = line.split('\t').collect();
        if raw_fields.len() != columns.len() {
            return Err(format!(
                "row has {} fields, expected {}",
                raw_fields.len(),
                columns.len()
            ));
        }
        let values = raw_fields
            .into_iter()
            .map(super::escape::decode_field)
            .collect();
        Ok(Row { columns, values })
    }

    pub fn get(&self, col: &str) -> Option<&str> {
        let idx = self.columns.iter().position(|c| c == col)?;
        self.values[idx].as_deref()
    }

    pub fn as_i64(&self, col: &str) -> Option<i64> {
        self.get(col)?.parse().ok()
    }

    pub fn as_i32(&self, col: &str) -> Option<i32> {
        self.get(col)?.parse().ok()
    }

    pub fn as_f64(&self, col: &str) -> Option<f64> {
        self.get(col)?.parse().ok()
    }

    pub fn as_date(&self, col: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(self.get(col)?, "%Y-%m-%d").ok()
    }

    pub fn as_bool(&self, col: &str) -> Option<bool> {
        match self.get(col)? {
            "t" => Some(true),
            "f" => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_copy_header() {
        let h = parse_copy_header(
            "COPY public.creance (id, numero_creance, montant_initial) FROM stdin;",
        )
        .unwrap();
        assert_eq!(h.table, "creance");
        assert_eq!(h.columns, vec!["id", "numero_creance", "montant_initial"]);
    }

    #[test]
    fn header_with_extra_whitespace() {
        let h =
            parse_copy_header("COPY public.uge  ( id ,  num_uge ,  libelle ) FROM stdin;").unwrap();
        assert_eq!(h.table, "uge");
        assert_eq!(h.columns, vec!["id", "num_uge", "libelle"]);
    }

    #[test]
    fn non_public_schema_returns_none() {
        assert!(parse_copy_header("COPY pg_catalog.foo (id) FROM stdin;").is_none());
    }

    #[test]
    fn unrelated_line_returns_none() {
        assert!(parse_copy_header("CREATE TABLE public.foo (id integer);").is_none());
        assert!(parse_copy_header("").is_none());
        assert!(parse_copy_header("-- comment").is_none());
    }

    #[test]
    fn copy_with_quoted_column_names() {
        let h = parse_copy_header(r#"COPY public.t (id, "order") FROM stdin;"#).unwrap();
        assert_eq!(h.columns, vec!["id", "order"]);
    }
}

#[cfg(test)]
mod row_tests {
    use super::*;

    fn cols() -> Vec<String> {
        vec![
            "id".into(),
            "name".into(),
            "amount".into(),
            "born".into(),
            "active".into(),
        ]
    }

    #[test]
    fn parse_row_basic() {
        let c = cols();
        let r = Row::parse(&c, "42\tAlice\t1234.56\t1990-05-13\tt").unwrap();
        assert_eq!(r.get("id"), Some("42"));
        assert_eq!(r.get("name"), Some("Alice"));
        assert_eq!(r.as_i64("id"), Some(42));
        assert_eq!(r.as_f64("amount"), Some(1234.56));
        assert_eq!(
            r.as_date("born"),
            Some(NaiveDate::from_ymd_opt(1990, 5, 13).unwrap())
        );
        assert_eq!(r.as_bool("active"), Some(true));
    }

    #[test]
    fn parse_row_with_nulls() {
        let c = cols();
        let r = Row::parse(&c, "1\t\\N\t\\N\t\\N\tf").unwrap();
        assert_eq!(r.get("name"), None);
        assert_eq!(r.as_f64("amount"), None);
        assert_eq!(r.as_date("born"), None);
        assert_eq!(r.as_bool("active"), Some(false));
    }

    #[test]
    fn parse_row_with_escaped_tab() {
        let c = vec!["a".into(), "b".into()];
        let r = Row::parse(&c, "x\\ty\tz").unwrap();
        assert_eq!(r.get("a"), Some("x\ty"));
        assert_eq!(r.get("b"), Some("z"));
    }

    #[test]
    fn parse_row_wrong_field_count_errors() {
        let c = cols();
        assert!(Row::parse(&c, "only\ttwo").is_err());
    }

    #[test]
    fn missing_column_returns_none() {
        let c = cols();
        let r = Row::parse(&c, "1\tBob\t10\t2000-01-01\tt").unwrap();
        assert_eq!(r.get("nonexistent"), None);
    }
}
