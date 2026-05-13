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
