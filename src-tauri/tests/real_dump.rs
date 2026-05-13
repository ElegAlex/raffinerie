//! Integration test against the real sucre_939.dump.
//! Skipped unless RAFFINERIE_REAL_DUMP env var points to a readable file.

use raffinerie::parser::parse;
use std::fs::File;
use std::time::Instant;

#[test]
fn parse_real_dump_if_available() {
    let path = match std::env::var("RAFFINERIE_REAL_DUMP") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("RAFFINERIE_REAL_DUMP not set, skipping.");
            return;
        }
    };
    let file = File::open(&path).expect("cannot open dump");
    let t0 = Instant::now();
    let mut last_pct: u8 = 0;
    let size = std::fs::metadata(&path).unwrap().len();
    let result = parse(file, |bytes| {
        let pct = ((bytes as f64 / size as f64) * 100.0) as u8;
        if pct >= last_pct + 10 {
            eprintln!("  {pct}%");
            last_pct = pct;
        }
    })
    .expect("parsing failed");
    let elapsed = t0.elapsed();
    eprintln!("== parsed in {:.2}s ==", elapsed.as_secs_f64());
    eprintln!("  creances:           {}", result.creances.len());
    eprintln!("  creances_regroupees:{}", result.creances_regroupees.len());
    eprintln!("  uges:               {}", result.uges.len());
    eprintln!("  etapes:             {}", result.etapes.len());
    eprintln!("  adresses:           {}", result.adresses.len());
    eprintln!("  corrupted_rows:     {}", result.corrupted_rows);

    assert!(result.creances.len() > 1000, "expected >1k creances");
    assert!(!result.uges.is_empty());
    assert!(elapsed.as_secs() < 30, "parsing too slow: {:?}", elapsed);
}
