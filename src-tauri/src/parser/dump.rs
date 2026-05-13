use crate::parser::copy_block::{parse_copy_header, Row};
use crate::schema::*;
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, Default)]
pub struct ParsedDump {
    pub creances: Vec<Creance>,
    pub creances_regroupees: Vec<CreanceRegroupee>,
    pub uges: Vec<Uge>,
    pub etapes: Vec<EtapeWorkflow>,
    pub adresses: Vec<AdresseDebiteur>,
    pub corrupted_rows: usize,
}

const USEFUL: &[&str] = &[
    "creance",
    "creance_regroupee",
    "uge",
    "etapeworkflow",
    "adresse_debiteur",
];

pub fn parse<R: Read>(reader: R, mut on_progress: impl FnMut(u64)) -> Result<ParsedDump, String> {
    let buf = BufReader::with_capacity(64 * 1024, reader);
    let mut out = ParsedDump::default();
    let mut current_table: Option<String> = None;
    let mut current_cols: Vec<String> = Vec::new();
    let mut bytes_read: u64 = 0;
    let mut progress_tick: u64 = 0;

    for line in buf.lines() {
        let line = line.map_err(|e| format!("read error: {e}"))?;
        bytes_read += line.len() as u64 + 1;
        if bytes_read - progress_tick > 5 * 1024 * 1024 {
            on_progress(bytes_read);
            progress_tick = bytes_read;
        }

        if current_table.is_none() {
            if let Some(h) = parse_copy_header(&line) {
                if USEFUL.contains(&h.table.as_str()) {
                    current_table = Some(h.table.clone());
                    current_cols = h.columns;
                } else {
                    current_table = Some("__skip__".into());
                    current_cols.clear();
                }
            }
            continue;
        }

        if line == "\\." {
            current_table = None;
            current_cols.clear();
            continue;
        }

        let table = current_table.as_deref().unwrap();
        if table == "__skip__" {
            continue;
        }

        let row = match Row::parse(&current_cols, &line) {
            Ok(r) => r,
            Err(_) => {
                out.corrupted_rows += 1;
                continue;
            }
        };

        let result = match table {
            "creance" => Creance::from_row(&row).map(|c| out.creances.push(c)),
            "creance_regroupee" => {
                CreanceRegroupee::from_row(&row).map(|c| out.creances_regroupees.push(c))
            }
            "uge" => Uge::from_row(&row).map(|u| out.uges.push(u)),
            "etapeworkflow" => EtapeWorkflow::from_row(&row).map(|e| out.etapes.push(e)),
            "adresse_debiteur" => AdresseDebiteur::from_row(&row).map(|a| out.adresses.push(a)),
            _ => Ok(()),
        };
        if result.is_err() {
            out.corrupted_rows += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_DUMP: &str = "\
-- header
SET something = 0;

COPY public.uge (id, num_uge, libelle) FROM stdin;
1\t9501\tROC indus
2\t9531\tROC bis
\\.

COPY public.unused (id, foo) FROM stdin;
99\tskip-me
\\.

COPY public.etapeworkflow (id, libelle) FROM stdin;
80\tNotification IND
\\.
";

    #[test]
    fn parses_useful_skips_unused() {
        let res = parse(MINI_DUMP.as_bytes(), |_| {}).unwrap();
        assert_eq!(res.uges.len(), 2);
        assert_eq!(res.uges[0].num_uge, "9501");
        assert_eq!(res.etapes.len(), 1);
        assert_eq!(res.creances.len(), 0);
        assert_eq!(res.corrupted_rows, 0);
    }

    #[test]
    fn corrupted_row_counted_not_fatal() {
        let bad = "\
COPY public.uge (id, num_uge, libelle) FROM stdin;
1\t9501\tROC
not\tenough
\\.
";
        let res = parse(bad.as_bytes(), |_| {}).unwrap();
        assert_eq!(res.uges.len(), 1);
        assert_eq!(res.corrupted_rows, 1);
    }

    #[test]
    fn progress_callback_invoked_at_least_once_on_large_input() {
        // Build a ~6 MB string by repeating a valid uge row
        let mut input = String::from("COPY public.uge (id, num_uge, libelle) FROM stdin;\n");
        for i in 0..300_000 {
            input.push_str(&format!("{i}\t{:04}\tlibelle\n", i % 10000));
        }
        input.push_str("\\.\n");
        let mut ticks = 0;
        let res = parse(input.as_bytes(), |_| ticks += 1).unwrap();
        assert!(
            ticks >= 1,
            "progress should fire at least once for >5MB input"
        );
        assert_eq!(res.uges.len(), 300_000);
    }
}
