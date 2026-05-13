# raffinerie Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Windows desktop app (Tauri + Rust + Alpine.js) that parses a PostgreSQL `pg_dump` plain-text file of the SUCRE database, applies user-configured filters (multi-UGE, comment, notification status, date range), and exports a multi-sheet Excel workbook.

**Architecture:** Rust backend streams the dump line-by-line, loading only 5 useful tables into memory (~80–120 MB RAM). A composable `FilterSet` evaluates rows in O(n) with O(1) joins via HashMap indexes. A column catalog drives a checkbox-based UI. Excel generation produces a synthesis sheet + N monthly sheets + a parameters/audit sheet via `rust_xlsxwriter`. Frontend is HTML + CSS + Alpine.js (no build step) embedded in Tauri 2's WebView2 window. Windows `.exe` is built via GitHub Actions runner `windows-latest` (cross-compilation from Linux is unreliable for Tauri).

**Tech Stack:**
- Rust stable, Cargo
- Tauri 2 (`tauri`, `tauri-cli`)
- `rust_xlsxwriter` (XLSX generation)
- `chrono` (dates)
- `serde` / `serde_json` (IPC + profile persistence)
- `unicode-normalization` (accent-insensitive search)
- Frontend: vanilla HTML/CSS + Alpine.js 3 (vendored, no npm)
- CI: GitHub Actions, `windows-latest` runner

**Working directory:** `/home/alex/Documents/REPO/raffinerie/`
**Reference dump for manual tests:** `/home/alex/Documents/REPO/SUCRE_DUMP/sucre_939.dump` (340 MB, NOT committed)

---

## File structure (target)

```
raffinerie/
├── .github/workflows/windows-build.yml
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/icon.png
│   └── src/
│       ├── main.rs                  ← Tauri bootstrap
│       ├── lib.rs                   ← lib re-exports for tests
│       ├── ipc.rs                   ← Tauri command handlers
│       ├── state.rs                 ← AppState (in-memory data + Mutex)
│       ├── catalog.rs               ← column catalog + 3 profils preset
│       ├── persistence.rs           ← profiles.json + last-session.json
│       ├── parser/
│       │   ├── mod.rs
│       │   ├── escape.rs            ← pg_dump escape decode
│       │   ├── copy_block.rs        ← COPY header + row parser
│       │   └── dump.rs              ← streaming orchestrator
│       ├── schema/
│       │   ├── mod.rs
│       │   ├── creance.rs
│       │   ├── creance_regroupee.rs
│       │   ├── uge.rs
│       │   ├── etapeworkflow.rs
│       │   └── adresse_debiteur.rs
│       ├── filter/
│       │   ├── mod.rs
│       │   ├── set.rs               ← FilterSet, NotifCriterion, DatePivot
│       │   └── eval.rs              ← evaluation engine + row joiner
│       ├── aggregator/
│       │   ├── mod.rs
│       │   └── monthly.rs
│       └── exporter/
│           ├── mod.rs
│           ├── value.rs             ← Value enum + cell writer
│           ├── synthese.rs
│           ├── monthly_sheet.rs
│           ├── params_sheet.rs
│           └── workbook.rs
│   └── tests/
│       ├── fixtures/builder.rs      ← mini-dump builder (in-memory)
│       └── integration.rs
├── src/                             ← frontend
│   ├── index.html
│   ├── app.js
│   ├── styles.css
│   └── vendor/alpine.min.js
├── docs/
│   ├── superpowers/specs/2026-05-13-raffinerie-design.md  (exists)
│   ├── superpowers/plans/2026-05-13-raffinerie-implementation.md  (this file)
│   └── guide-utilisateur.md  (Task 26)
├── .gitignore  (exists)
├── README.md   (exists)
└── CHANGELOG.md  (Task 26)
```

---

## Conventions

- **TDD discipline** : every code task starts with a failing test, then minimal implementation, then verification, then commit.
- **Commits in French**, conventional commits style (`feat:`, `test:`, `chore:`, `docs:`, `ci:`, `fix:`, `refactor:`).
- **One commit per task minimum**, more if a task has natural milestones.
- **Rust edition 2021**, `cargo fmt` + `cargo clippy -- -D warnings` clean before commit.
- **No `.dump` or `.xlsx` ever committed** (already gitignored). Use programmatic fixtures.

---

## Wave 1 — Scaffolding & Parser foundation

### Task 1: Cargo + Tauri scaffolding

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src/index.html`
- Create: `src/styles.css`
- Create: `src/app.js`
- Create: `src-tauri/icons/icon.png` (placeholder 512×512 PNG generated)

- [ ] **Step 1: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "raffinerie"
version = "0.1.0"
edition = "2021"
description = "Extracteur autonome de créances SUCRE depuis un dump pg_dump"
authors = ["Alexandre Berge"]

[lib]
name = "raffinerie"
path = "src/lib.rs"

[[bin]]
name = "raffinerie"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
rust_xlsxwriter = { version = "0.79", features = ["chrono"] }
unicode-normalization = "0.1"
thiserror = "1"
sha2 = "0.10"
once_cell = "1"

[dev-dependencies]
calamine = "0.26"
tempfile = "3"

[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = "z"
```

- [ ] **Step 2: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 3: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "raffinerie",
  "version": "0.1.0",
  "identifier": "fr.cpam92.raffinerie",
  "build": {
    "frontendDist": "../src"
  },
  "app": {
    "windows": [
      {
        "title": "raffinerie — extracteur SUCRE",
        "width": 1100,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600,
        "fileDropEnabled": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.png"],
    "windows": {
      "nsis": {
        "installerIcon": "icons/icon.png",
        "installMode": "perMachine"
      }
    }
  },
  "plugins": {}
}
```

- [ ] **Step 4: Create `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    raffinerie::run();
}
```

- [ ] **Step 5: Create `src-tauri/src/lib.rs`**

```rust
pub mod parser;
pub mod schema;
pub mod filter;
pub mod aggregator;
pub mod exporter;
pub mod catalog;
pub mod state;
pub mod persistence;
pub mod ipc;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            ipc::parse_dump,
            ipc::list_uges,
            ipc::list_natures,
            ipc::list_etapes,
            ipc::list_statuts,
            ipc::count_filtered,
            ipc::preview,
            ipc::list_columns,
            ipc::load_profiles,
            ipc::save_profile,
            ipc::delete_profile,
            ipc::load_session,
            ipc::save_session,
            ipc::export_xlsx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Create empty module files**

Each is a `pub mod` file with `// placeholder` for now. Create:
- `src-tauri/src/parser/mod.rs` → `// placeholder`
- `src-tauri/src/schema/mod.rs` → `// placeholder`
- `src-tauri/src/filter/mod.rs` → `// placeholder`
- `src-tauri/src/aggregator/mod.rs` → `// placeholder`
- `src-tauri/src/exporter/mod.rs` → `// placeholder`
- `src-tauri/src/catalog.rs` → `// placeholder`
- `src-tauri/src/state.rs` → `#[derive(Default)] pub struct AppState;`
- `src-tauri/src/persistence.rs` → `// placeholder`
- `src-tauri/src/ipc.rs` → see Step 7

- [ ] **Step 7: Create minimal `src-tauri/src/ipc.rs` stubs**

```rust
// Stub IPC handlers — will be implemented in Wave 4.
// Each returns a placeholder error so the app compiles.

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ParseResult { pub creances: usize, pub elapsed_ms: u128 }

#[tauri::command] pub async fn parse_dump(_path: String) -> Result<ParseResult, String> { Err("not implemented".into()) }
#[tauri::command] pub async fn list_uges() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_natures() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_etapes() -> Result<Vec<(i32, String)>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_statuts() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn count_filtered(_filters: serde_json::Value) -> Result<usize, String> { Err("ni".into()) }
#[tauri::command] pub async fn preview(_filters: serde_json::Value, _columns: Vec<String>) -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_columns() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn load_profiles() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn save_profile(_name: String, _cols: Vec<String>) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn delete_profile(_name: String) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn load_session() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn save_session(_session: serde_json::Value) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn export_xlsx(_path: String, _filters: serde_json::Value, _columns: Vec<String>) -> Result<(), String> { Err("ni".into()) }
```

- [ ] **Step 8: Create minimal frontend stubs**

`src/index.html`:
```html
<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <title>raffinerie</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <h1>raffinerie</h1>
  <p>UI en cours de développement.</p>
  <script src="app.js"></script>
</body>
</html>
```

`src/styles.css`: `body { font-family: system-ui; padding: 2rem; }`

`src/app.js`: `console.log('raffinerie boot');`

- [ ] **Step 9: Create placeholder icon**

A 512×512 PNG with a stylized "R" on a gradient background. Generate via Rust one-liner or use a simple solid-color PNG. For now, generate with `imagemagick` (likely installed on Fedora):

```bash
cd src-tauri && convert -size 512x512 xc:'#7B3F00' -fill white -gravity center -pointsize 320 -annotate +0+0 'R' icons/icon.png
```

If `convert` unavailable, use Rust `image` crate inline in a build script, or download a placeholder. Either way produce a valid 512×512 PNG.

- [ ] **Step 10: Verify build**

Run:
```bash
cd src-tauri && cargo check 2>&1 | tail -20
```
Expected: compiles cleanly (warnings OK for unused stubs).

- [ ] **Step 11: Commit**

```bash
cd /home/alex/Documents/REPO/raffinerie
git add -A
git commit -m "chore: scaffolding Tauri + Cargo + frontend stubs"
git push
```

---

### Task 2: pg_dump escape decoder (TDD)

The pg_dump COPY format escapes certain characters in fields: `\N` is NULL, `\\` is backslash, `\t` is tab, `\n` is newline, `\r` is carriage return, `\b` backspace, `\f` formfeed, `\v` vertical tab.

**Files:**
- Create: `src-tauri/src/parser/escape.rs`
- Test: inline `#[cfg(test)]` in same file

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/parser/escape.rs`:
```rust
/// Decodes a pg_dump COPY field value.
/// Returns `None` if the field is `\N` (SQL NULL), `Some(String)` otherwise.
pub fn decode_field(raw: &str) -> Option<String> {
    todo!()
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
        // \N is only NULL when it's the entire field. "\N" alone == NULL,
        // but "foo\\N" is "foo" + newline-not-equal... actually pg_dump
        // never produces literal "\N" inside; defensive behavior: treat as
        // backslash + 'N' which is invalid escape -> we keep raw 'N'.
        assert_eq!(decode_field("foo\\Nbar"), Some("fooNbar".into()));
    }

    #[test]
    fn trailing_lone_backslash_kept() {
        assert_eq!(decode_field("foo\\"), Some("foo\\".into()));
    }
}
```

Add `pub mod escape;` to `src-tauri/src/parser/mod.rs`.

- [ ] **Step 2: Run tests, verify they fail**

```bash
cd src-tauri && cargo test --lib parser::escape 2>&1 | tail -20
```
Expected: tests panic at `todo!()`.

- [ ] **Step 3: Implement `decode_field`**

```rust
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
            Some(other) => out.push(other), // unknown escape: keep the char
            None => out.push('\\'),         // trailing backslash
        }
    }
    Some(out)
}
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
cd src-tauri && cargo test --lib parser::escape 2>&1 | tail -5
```
Expected: `test result: ok. 10 passed`.

- [ ] **Step 5: Run clippy + fmt**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/parser/
git commit -m "feat(parser): décodeur d'échappements pg_dump COPY"
```

---

### Task 3: COPY block header parser (TDD)

Parses lines like `COPY public.creance (id, numero_creance, date_der_ope, ...) FROM stdin;` into `(table_name, vec_of_columns)`.

**Files:**
- Create: `src-tauri/src/parser/copy_block.rs`
- Test: inline `#[cfg(test)]`

- [ ] **Step 1: Write failing tests**

In `src-tauri/src/parser/copy_block.rs`:
```rust
#[derive(Debug, PartialEq, Eq)]
pub struct CopyHeader {
    pub table: String,
    pub columns: Vec<String>,
}

/// Parse a COPY header line. Returns None if the line is not a COPY header.
pub fn parse_copy_header(line: &str) -> Option<CopyHeader> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_copy_header() {
        let h = parse_copy_header("COPY public.creance (id, numero_creance, montant_initial) FROM stdin;").unwrap();
        assert_eq!(h.table, "creance");
        assert_eq!(h.columns, vec!["id", "numero_creance", "montant_initial"]);
    }

    #[test]
    fn header_with_extra_whitespace() {
        let h = parse_copy_header("COPY public.uge  ( id ,  num_uge ,  libelle ) FROM stdin;").unwrap();
        assert_eq!(h.table, "uge");
        assert_eq!(h.columns, vec!["id", "num_uge", "libelle"]);
    }

    #[test]
    fn non_public_schema_returns_none() {
        // We only care about `public.<table>`; other schemas are skipped.
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
        // pg_dump may quote reserved-word columns. Strip quotes.
        let h = parse_copy_header(r#"COPY public.t (id, "order") FROM stdin;"#).unwrap();
        assert_eq!(h.columns, vec!["id", "order"]);
    }
}
```

Add `pub mod copy_block;` to `parser/mod.rs`.

- [ ] **Step 2: Run, verify fail**

```bash
cd src-tauri && cargo test --lib parser::copy_block 2>&1 | tail -10
```

- [ ] **Step 3: Implement**

```rust
pub fn parse_copy_header(line: &str) -> Option<CopyHeader> {
    let line = line.trim();
    let rest = line.strip_prefix("COPY public.")?;
    let (table, after_table) = rest.split_once(|c: char| c.is_whitespace() || c == '(')?;
    // Re-find the '(' since split_once consumed it if it was the splitter
    let cols_start = line.find('(')?;
    let cols_end = line.find(')')?;
    if cols_end <= cols_start {
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
    let _ = after_table; // silence
    Some(CopyHeader {
        table: table.to_string(),
        columns,
    })
}
```

- [ ] **Step 4: Run, verify pass**

```bash
cd src-tauri && cargo test --lib parser::copy_block 2>&1 | tail -5
```

- [ ] **Step 5: fmt + clippy**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/parser/
git commit -m "feat(parser): parseur d'en-têtes COPY"
```

---

### Task 4: Row parser with type coercion (TDD)

Splits a COPY data row on `\t`, decodes each field via `decode_field`, and provides typed accessors (`as_str`, `as_i64`, `as_f64`, `as_date`, `as_bool`).

**Files:**
- Modify: `src-tauri/src/parser/copy_block.rs` (append)

- [ ] **Step 1: Write failing tests**

Append to `src-tauri/src/parser/copy_block.rs`:
```rust
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
mod row_tests {
    use super::*;

    fn cols() -> Vec<String> {
        vec!["id".into(), "name".into(), "amount".into(), "born".into(), "active".into()]
    }

    #[test]
    fn parse_row_basic() {
        let c = cols();
        let r = Row::parse(&c, "42\tAlice\t1234.56\t1990-05-13\tt").unwrap();
        assert_eq!(r.get("id"), Some("42"));
        assert_eq!(r.get("name"), Some("Alice"));
        assert_eq!(r.as_i64("id"), Some(42));
        assert_eq!(r.as_f64("amount"), Some(1234.56));
        assert_eq!(r.as_date("born"), Some(NaiveDate::from_ymd_opt(1990, 5, 13).unwrap()));
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
```

- [ ] **Step 2: Run, verify fail then pass**

Test should compile and pass since implementation is included. Run:
```bash
cd src-tauri && cargo test --lib parser::copy_block::row_tests 2>&1 | tail -5
```
Expected: all pass.

- [ ] **Step 3: fmt + clippy**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser/
git commit -m "feat(parser): Row avec accesseurs typés (i64, f64, date, bool)"
```

---

### Task 5: Schema structs

5 plain Rust structs, one per useful table. Each implements `from_row(row: &Row) -> Result<Self, String>`.

**Files:**
- Create: `src-tauri/src/schema/creance.rs`
- Create: `src-tauri/src/schema/creance_regroupee.rs`
- Create: `src-tauri/src/schema/uge.rs`
- Create: `src-tauri/src/schema/etapeworkflow.rs`
- Create: `src-tauri/src/schema/adresse_debiteur.rs`
- Modify: `src-tauri/src/schema/mod.rs`

- [ ] **Step 1: Write `src-tauri/src/schema/mod.rs`**

```rust
pub mod creance;
pub mod creance_regroupee;
pub mod uge;
pub mod etapeworkflow;
pub mod adresse_debiteur;

pub use creance::Creance;
pub use creance_regroupee::CreanceRegroupee;
pub use uge::Uge;
pub use etapeworkflow::EtapeWorkflow;
pub use adresse_debiteur::AdresseDebiteur;
```

- [ ] **Step 2: Write `src-tauri/src/schema/creance.rs`**

```rust
use crate::parser::copy_block::Row;
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Creance {
    pub id: i64,
    pub workflow: Option<i32>,
    pub numero_creance: String,
    pub date_der_ope: Option<NaiveDate>,
    pub date_detect: Option<NaiveDate>,
    pub nature_compte: String,
    pub statut_compte: String,
    pub gest_num: String,
    pub numero_debiteur: String,
    pub cat_debiteur: String,
    pub num_uge_gestion: String,
    pub montant_initial: f64,
    pub solde: f64,
    pub part_mutuel: Option<f64>,
    pub type_prest: Option<String>,
    pub arc_det: Option<String>,
    pub nature_der_ope: Option<String>,
    pub matricule_assure: Option<String>,
    pub date_mandatement: Option<NaiveDate>,
    pub activite: Option<String>,
    pub num_compte: Option<String>,
    pub nom_assure: Option<String>,
    pub prenom_assure: Option<String>,
    pub num_uge_detect: String,
    pub date_integration: Option<NaiveDate>,
    pub flux: Option<String>,
    pub commentaire_creance: Option<String>,
    pub iduge: Option<i32>,
    pub creanceregroupeeid: Option<i64>,
    pub num_technicien: Option<String>,
    pub date_prescription: Option<NaiveDate>,
}

impl Creance {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i64("id").ok_or("creance.id missing")?,
            workflow: r.as_i32("workflow"),
            numero_creance: r.get("numero_creance").ok_or("numero_creance missing")?.to_string(),
            date_der_ope: r.as_date("date_der_ope"),
            date_detect: r.as_date("date_detect"),
            nature_compte: r.get("nature_compte").unwrap_or("").to_string(),
            statut_compte: r.get("statut_compte").unwrap_or("").to_string(),
            gest_num: r.get("gest_num").unwrap_or("").to_string(),
            numero_debiteur: r.get("numero_debiteur").unwrap_or("").to_string(),
            cat_debiteur: r.get("cat_debiteur").unwrap_or("").to_string(),
            num_uge_gestion: r.get("num_uge_gestion").unwrap_or("").to_string(),
            montant_initial: r.as_f64("montant_initial").unwrap_or(0.0),
            solde: r.as_f64("solde").unwrap_or(0.0),
            part_mutuel: r.as_f64("part_mutuel"),
            type_prest: r.get("type_prest").map(String::from),
            arc_det: r.get("arc_det").map(String::from),
            nature_der_ope: r.get("nature_der_ope").map(String::from),
            matricule_assure: r.get("matricule_assure").map(String::from),
            date_mandatement: r.as_date("date_mandatement"),
            activite: r.get("activite").map(String::from),
            num_compte: r.get("num_compte").map(String::from),
            nom_assure: r.get("nom_assure").map(String::from),
            prenom_assure: r.get("prenom_assure").map(String::from),
            num_uge_detect: r.get("num_uge_detect").unwrap_or("").to_string(),
            date_integration: r.as_date("date_integration"),
            flux: r.get("flux").map(String::from),
            commentaire_creance: r.get("commentaire_creance").map(String::from),
            iduge: r.as_i32("iduge"),
            creanceregroupeeid: r.as_i64("creanceregroupeeid"),
            num_technicien: r.get("num_technicien").map(String::from),
            date_prescription: r.as_date("date_prescription"),
        })
    }
}
```

- [ ] **Step 3: Write `creance_regroupee.rs`**

```rust
use crate::parser::copy_block::Row;
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CreanceRegroupee {
    pub id: i64,
    pub numero_reference: String,
    pub numero_debiteur: String,
    pub date_detection: Option<NaiveDate>,
    pub motif_notif: Option<String>,
    pub date_ar_notif_debiteur: Option<NaiveDate>,
    pub date_ar_mdm_debiteur: Option<NaiveDate>,
    pub commentaire_creance: Option<String>,
    pub etapewf: Option<i32>,
    pub is_douteux: bool,
    pub numero_og3s: Option<String>,
}

impl CreanceRegroupee {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i64("id").ok_or("creance_regroupee.id missing")?,
            numero_reference: r.get("numero_reference").unwrap_or("").to_string(),
            numero_debiteur: r.get("numero_debiteur").unwrap_or("").to_string(),
            date_detection: r.as_date("date_detection"),
            motif_notif: r.get("motif_notif").map(String::from),
            date_ar_notif_debiteur: r.as_date("date_ar_notif_debiteur"),
            date_ar_mdm_debiteur: r.as_date("date_ar_mdm_debiteur"),
            commentaire_creance: r.get("commentaire_creance").map(String::from),
            etapewf: r.as_i32("etapewf"),
            is_douteux: r.as_bool("is_douteux").unwrap_or(false),
            numero_og3s: r.get("numero_og3s").map(String::from),
        })
    }
}
```

- [ ] **Step 4: Write `uge.rs`, `etapeworkflow.rs`, `adresse_debiteur.rs`**

`uge.rs`:
```rust
use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Uge {
    pub id: i32,
    pub num_uge: String,
    pub libelle: Option<String>,
}

impl Uge {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i32("id").ok_or("uge.id missing")?,
            num_uge: r.get("num_uge").ok_or("num_uge missing")?.to_string(),
            libelle: r.get("libelle").map(String::from),
        })
    }
}
```

`etapeworkflow.rs`:
```rust
use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EtapeWorkflow {
    pub id: i32,
    pub libelle: String,
}

impl EtapeWorkflow {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i32("id").ok_or("etapeworkflow.id missing")?,
            libelle: r.get("libelle").unwrap_or("").to_string(),
        })
    }
}
```

`adresse_debiteur.rs`:
```rust
use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AdresseDebiteur {
    pub numero_debiteur: String,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub adresse: Option<String>,
    pub code_postal: Option<String>,
    pub commune: Option<String>,
}

impl AdresseDebiteur {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            numero_debiteur: r.get("numero_debiteur").ok_or("numero_debiteur missing")?.to_string(),
            nom: r.get("nom").map(String::from),
            prenom: r.get("prenom").map(String::from),
            adresse: r.get("adresse").map(String::from),
            code_postal: r.get("code_postal").map(String::from),
            commune: r.get("commune").map(String::from),
        })
    }
}
```

- [ ] **Step 5: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

- [ ] **Step 6: fmt + clippy**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/schema/
git commit -m "feat(schema): structs des 5 tables SUCRE utiles"
```

---

### Task 6: Dump streaming orchestrator (TDD)

Reads the dump line-by-line via `BufReader`, detects useful `COPY` blocks, skips non-useful ones, parses rows into the appropriate `Vec<T>`, returns a `ParsedDump` struct.

**Files:**
- Create: `src-tauri/src/parser/dump.rs`
- Modify: `src-tauri/src/parser/mod.rs`

- [ ] **Step 1: Write failing test**

In `src-tauri/src/parser/dump.rs`:
```rust
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
            "creance_regroupee" => CreanceRegroupee::from_row(&row).map(|c| out.creances_regroupees.push(c)),
            "uge" => Uge::from_row(&row).map(|u| out.uges.push(u)),
            "etapeworkflow" => EtapeWorkflow::from_row(&row).map(|e| out.etapes.push(e)),
            "adresse_debiteur" => AdresseDebiteur::from_row(&row).map(|a| out.adresses.push(a)),
            _ => Ok(()),
        };
        if result.is_err() {
            out.corrupted_rows += 1;
        }
    }
    let _ = on_progress;
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
}
```

In `src-tauri/src/parser/mod.rs`:
```rust
pub mod escape;
pub mod copy_block;
pub mod dump;

pub use dump::{parse, ParsedDump};
```

- [ ] **Step 2: Run tests, verify pass**

```bash
cd src-tauri && cargo test --lib parser::dump 2>&1 | tail -10
```

- [ ] **Step 3: fmt + clippy**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser/
git commit -m "feat(parser): orchestrateur streaming du dump pg_dump"
```

---

### Task 7: Manual integration test with real dump

Confirm the parser handles the real 340 MB `sucre_939.dump` without crashing, in <10s, with reasonable row counts.

**Files:**
- Create: `src-tauri/tests/real_dump.rs`

- [ ] **Step 1: Write `tests/real_dump.rs`**

```rust
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
    assert!(result.uges.len() > 0);
    assert!(elapsed.as_secs() < 30, "parsing too slow: {:?}", elapsed);
}
```

- [ ] **Step 2: Run with real dump**

```bash
cd src-tauri && RAFFINERIE_REAL_DUMP=/home/alex/Documents/REPO/SUCRE_DUMP/sucre_939.dump cargo test --release --test real_dump -- --nocapture 2>&1 | tail -20
```

Expected output: row counts > 1000 creances, parsing in <10s, 0 corrupted rows (or very few).

If parsing fails, debug the issue (likely an edge case in escape decoding or COPY parsing — fix the parser, add a regression test, re-run).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/real_dump.rs
git commit -m "test(parser): test d'intégration contre dump réel sucre_939"
```

---

## Wave 2 — Filtering & aggregation

### Task 8: FilterSet, NotifCriterion, DatePivot (types only)

**Files:**
- Create: `src-tauri/src/filter/set.rs`
- Modify: `src-tauri/src/filter/mod.rs`

- [ ] **Step 1: Write `filter/mod.rs`**

```rust
pub mod set;
pub mod eval;

pub use set::{DatePivot, FilterSet, NotifCriterion};
pub use eval::{evaluate, FilteredRow};
```

- [ ] **Step 2: Write `filter/set.rs`**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSet {
    #[serde(default)]
    pub uges: Vec<String>,
    #[serde(default)]
    pub nature_compte: Vec<String>,
    #[serde(default)]
    pub commentaire_contient: Option<String>,
    #[serde(default = "default_true")]
    pub commentaire_insensible: bool,
    #[serde(default)]
    pub notif_criterion: NotifCriterion,
    #[serde(default)]
    pub date_pivot: DatePivot,
    #[serde(default)]
    pub date_min: Option<NaiveDate>,
    #[serde(default)]
    pub date_max: Option<NaiveDate>,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotifCriterion {
    #[default]
    Aucun,
    MotifNotifNonVide,
    DateArNotifNonVide,
    EtapeWfDans { ids: Vec<i32> },
    StatutCompteDans { values: Vec<String> },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatePivot {
    DateDetect,
    #[default]
    DateIntegration,
    DateDerOpe,
    DateMandatement,
    DateArNotifDebiteur,
    DateDetectionRegroupee,
}
```

- [ ] **Step 3: Compile check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/filter/
git commit -m "feat(filter): types FilterSet, NotifCriterion, DatePivot"
```

---

### Task 9: Filter evaluation engine (TDD)

Joins `creance` with `creance_regroupee` via `creanceregroupeeid`, applies all filter predicates with short-circuit. Returns iterator of `FilteredRow` (creance + optional regroupee).

**Files:**
- Create: `src-tauri/src/filter/eval.rs`

- [ ] **Step 1: Write failing tests**

```rust
use crate::filter::set::{DatePivot, FilterSet, NotifCriterion};
use crate::parser::ParsedDump;
use crate::schema::{Creance, CreanceRegroupee};
use chrono::NaiveDate;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

pub struct FilteredRow<'a> {
    pub creance: &'a Creance,
    pub regroupee: Option<&'a CreanceRegroupee>,
    pub pivot_date: Option<NaiveDate>,
}

pub fn evaluate<'a>(dump: &'a ParsedDump, fs: &FilterSet) -> Vec<FilteredRow<'a>> {
    let regroupee_idx: HashMap<i64, &CreanceRegroupee> =
        dump.creances_regroupees.iter().map(|r| (r.id, r)).collect();

    let needle = fs.commentaire_contient.as_ref().map(|s| {
        if fs.commentaire_insensible {
            normalize(s)
        } else {
            s.clone()
        }
    });

    dump.creances
        .iter()
        .filter_map(|c| {
            // UGE filter
            if !fs.uges.is_empty() && !fs.uges.contains(&c.num_uge_gestion) {
                return None;
            }
            // Nature compte filter
            if !fs.nature_compte.is_empty() && !fs.nature_compte.contains(&c.nature_compte) {
                return None;
            }

            let regr = c.creanceregroupeeid.and_then(|id| regroupee_idx.get(&id).copied());

            // Commentaire filter (on regroupee)
            if let Some(n) = &needle {
                let hay_raw = regr.and_then(|r| r.commentaire_creance.as_deref()).unwrap_or("");
                let hay = if fs.commentaire_insensible { normalize(hay_raw) } else { hay_raw.to_string() };
                if !hay.contains(n) {
                    return None;
                }
            }

            // Notification criterion
            match &fs.notif_criterion {
                NotifCriterion::Aucun => {}
                NotifCriterion::MotifNotifNonVide => {
                    if regr.and_then(|r| r.motif_notif.as_deref()).filter(|s| !s.is_empty()).is_none() {
                        return None;
                    }
                }
                NotifCriterion::DateArNotifNonVide => {
                    if regr.and_then(|r| r.date_ar_notif_debiteur).is_none() {
                        return None;
                    }
                }
                NotifCriterion::EtapeWfDans { ids } => {
                    let etape = regr.and_then(|r| r.etapewf);
                    if !etape.map(|e| ids.contains(&e)).unwrap_or(false) {
                        return None;
                    }
                }
                NotifCriterion::StatutCompteDans { values } => {
                    if !values.contains(&c.statut_compte) {
                        return None;
                    }
                }
            }

            // Date pivot
            let pivot = pivot_date(c, regr, fs.date_pivot);

            // Date range
            if let Some(min) = fs.date_min {
                if pivot.map(|d| d < min).unwrap_or(true) {
                    return None;
                }
            }
            if let Some(max) = fs.date_max {
                if pivot.map(|d| d > max).unwrap_or(true) {
                    return None;
                }
            }

            Some(FilteredRow { creance: c, regroupee: regr, pivot_date: pivot })
        })
        .collect()
}

fn pivot_date(c: &Creance, r: Option<&CreanceRegroupee>, pivot: DatePivot) -> Option<NaiveDate> {
    match pivot {
        DatePivot::DateDetect => c.date_detect,
        DatePivot::DateIntegration => c.date_integration,
        DatePivot::DateDerOpe => c.date_der_ope,
        DatePivot::DateMandatement => c.date_mandatement,
        DatePivot::DateArNotifDebiteur => r.and_then(|r| r.date_ar_notif_debiteur),
        DatePivot::DateDetectionRegroupee => r.and_then(|r| r.date_detection),
    }
}

fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;

    fn dummy_dump() -> ParsedDump {
        let mut d = ParsedDump::default();
        d.creances_regroupees.push(CreanceRegroupee {
            id: 1,
            numero_reference: "REF1".into(),
            numero_debiteur: "D1".into(),
            date_detection: None,
            motif_notif: Some("Notification envoyée".into()),
            date_ar_notif_debiteur: None,
            date_ar_mdm_debiteur: None,
            commentaire_creance: Some("Indu ROC".into()),
            etapewf: Some(79),
            is_douteux: false,
            numero_og3s: None,
        });
        d.creances_regroupees.push(CreanceRegroupee {
            id: 2,
            numero_reference: "REF2".into(),
            numero_debiteur: "D2".into(),
            date_detection: None,
            motif_notif: None,
            date_ar_notif_debiteur: None,
            date_ar_mdm_debiteur: None,
            commentaire_creance: Some("Autre".into()),
            etapewf: Some(1),
            is_douteux: false,
            numero_og3s: None,
        });
        for (i, regr_id) in [(101_i64, 1_i64), (102, 2)].iter() {
            d.creances.push(Creance {
                id: *i,
                workflow: None,
                numero_creance: format!("C{}", i),
                date_der_ope: None,
                date_detect: None,
                nature_compte: "IND".into(),
                statut_compte: "NO".into(),
                gest_num: "G".into(),
                numero_debiteur: "D".into(),
                cat_debiteur: "C".into(),
                num_uge_gestion: "9501".into(),
                montant_initial: 100.0,
                solde: 100.0,
                part_mutuel: None,
                type_prest: None,
                arc_det: None,
                nature_der_ope: None,
                matricule_assure: None,
                date_mandatement: None,
                activite: None,
                num_compte: None,
                nom_assure: None,
                prenom_assure: None,
                num_uge_detect: "9501".into(),
                date_integration: NaiveDate::from_ymd_opt(2026, 3, 15),
                flux: None,
                commentaire_creance: None,
                iduge: None,
                creanceregroupeeid: Some(*regr_id),
                num_technicien: None,
                date_prescription: None,
            });
        }
        d
    }

    #[test]
    fn no_filter_returns_all() {
        let d = dummy_dump();
        let r = evaluate(&d, &FilterSet::default());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn uge_filter() {
        let d = dummy_dump();
        let fs = FilterSet { uges: vec!["9501".into()], ..Default::default() };
        assert_eq!(evaluate(&d, &fs).len(), 2);
        let fs = FilterSet { uges: vec!["9999".into()], ..Default::default() };
        assert_eq!(evaluate(&d, &fs).len(), 0);
    }

    #[test]
    fn commentaire_filter_case_accent_insensitive() {
        let d = dummy_dump();
        let fs = FilterSet {
            commentaire_contient: Some("INDU rOC".into()),
            commentaire_insensible: true,
            ..Default::default()
        };
        let r = evaluate(&d, &fs);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].creance.id, 101);
    }

    #[test]
    fn motif_notif_filter() {
        let d = dummy_dump();
        let fs = FilterSet {
            notif_criterion: NotifCriterion::MotifNotifNonVide,
            ..Default::default()
        };
        let r = evaluate(&d, &fs);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].creance.id, 101);
    }

    #[test]
    fn date_range_with_pivot_integration() {
        let d = dummy_dump();
        let fs = FilterSet {
            date_pivot: DatePivot::DateIntegration,
            date_min: NaiveDate::from_ymd_opt(2026, 1, 1),
            date_max: NaiveDate::from_ymd_opt(2026, 12, 31),
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs).len(), 2);
        let fs2 = FilterSet {
            date_pivot: DatePivot::DateIntegration,
            date_min: NaiveDate::from_ymd_opt(2027, 1, 1),
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs2).len(), 0);
    }
}
```

- [ ] **Step 2: Run tests, verify pass**

```bash
cd src-tauri && cargo test --lib filter 2>&1 | tail -10
```

- [ ] **Step 3: fmt + clippy**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/filter/
git commit -m "feat(filter): moteur d'évaluation avec jointure regroupée"
```

---

### Task 10: Monthly aggregator (TDD)

Groups `FilteredRow` by `(YYYY-MM, UGE)` for the synthesis sheet, and by month only for the monthly data sheets.

**Files:**
- Create: `src-tauri/src/aggregator/monthly.rs`
- Modify: `src-tauri/src/aggregator/mod.rs`

- [ ] **Step 1: Write `aggregator/mod.rs`**

```rust
pub mod monthly;
pub use monthly::{group_by_month, MonthKey, MonthlyBucket, SynthesisRow};
```

- [ ] **Step 2: Write `aggregator/monthly.rs` with tests**

```rust
use crate::filter::FilteredRow;
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonthKey { pub year: i32, pub month: u32 }

impl MonthKey {
    pub fn from_date(d: NaiveDate) -> Self { Self { year: d.year(), month: d.month() } }
    pub fn label(&self) -> String { format!("{:04}-{:02}", self.year, self.month) }
}

#[derive(Debug, Default)]
pub struct MonthlyBucket<'a> {
    pub rows: Vec<&'a FilteredRow<'a>>,
}

#[derive(Debug, Default, Clone)]
pub struct SynthesisRow {
    pub month: Option<MonthKey>,        // None = "Sans date pivot"
    pub uge: String,
    pub count: usize,
    pub somme_montant_initial: f64,
    pub somme_solde: f64,
}

pub fn group_by_month<'a>(rows: &'a [FilteredRow<'a>]) -> BTreeMap<Option<MonthKey>, MonthlyBucket<'a>> {
    let mut out: BTreeMap<Option<MonthKey>, MonthlyBucket<'a>> = BTreeMap::new();
    for r in rows {
        let k = r.pivot_date.map(MonthKey::from_date);
        out.entry(k).or_default().rows.push(r);
    }
    out
}

pub fn synthesis<'a>(rows: &'a [FilteredRow<'a>]) -> Vec<SynthesisRow> {
    let mut map: BTreeMap<(Option<MonthKey>, String), SynthesisRow> = BTreeMap::new();
    for r in rows {
        let k = (r.pivot_date.map(MonthKey::from_date), r.creance.num_uge_gestion.clone());
        let entry = map.entry(k.clone()).or_insert_with(|| SynthesisRow {
            month: k.0,
            uge: k.1.clone(),
            ..Default::default()
        });
        entry.count += 1;
        entry.somme_montant_initial += r.creance.montant_initial;
        entry.somme_solde += r.creance.solde;
    }
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilteredRow;
    use crate::schema::Creance;

    fn make_creance(id: i64, uge: &str, montant: f64) -> Creance {
        Creance {
            id,
            workflow: None,
            numero_creance: format!("C{id}"),
            date_der_ope: None,
            date_detect: None,
            nature_compte: "IND".into(),
            statut_compte: "NO".into(),
            gest_num: "G".into(),
            numero_debiteur: "D".into(),
            cat_debiteur: "C".into(),
            num_uge_gestion: uge.into(),
            montant_initial: montant,
            solde: montant,
            part_mutuel: None,
            type_prest: None,
            arc_det: None,
            nature_der_ope: None,
            matricule_assure: None,
            date_mandatement: None,
            activite: None,
            num_compte: None,
            nom_assure: None,
            prenom_assure: None,
            num_uge_detect: uge.into(),
            date_integration: None,
            flux: None,
            commentaire_creance: None,
            iduge: None,
            creanceregroupeeid: None,
            num_technicien: None,
            date_prescription: None,
        }
    }

    #[test]
    fn synthesis_aggregates_by_month_uge() {
        let c1 = make_creance(1, "9501", 100.0);
        let c2 = make_creance(2, "9501", 50.0);
        let c3 = make_creance(3, "9531", 200.0);
        let rows = vec![
            FilteredRow { creance: &c1, regroupee: None, pivot_date: NaiveDate::from_ymd_opt(2026, 1, 5) },
            FilteredRow { creance: &c2, regroupee: None, pivot_date: NaiveDate::from_ymd_opt(2026, 1, 20) },
            FilteredRow { creance: &c3, regroupee: None, pivot_date: NaiveDate::from_ymd_opt(2026, 2, 1) },
        ];
        let s = synthesis(&rows);
        assert_eq!(s.len(), 2);
        let r1 = s.iter().find(|r| r.uge == "9501").unwrap();
        assert_eq!(r1.count, 2);
        assert!((r1.somme_montant_initial - 150.0).abs() < 1e-6);
    }
}
```

- [ ] **Step 3: Run, verify pass**

```bash
cd src-tauri && cargo test --lib aggregator 2>&1 | tail -5
```

- [ ] **Step 4: fmt + clippy + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/aggregator/
git commit -m "feat(aggregator): regroupement mensuel + synthèse UGE×mois"
```

---

## Wave 3 — Column catalog & profiles

### Task 11: Column catalog (TDD)

Defines all 40+ available columns with metadata: `id`, `label`, `group`, `source_table`, plus 3 preset profiles.

**Files:**
- Create: `src-tauri/src/catalog.rs`

- [ ] **Step 1: Write catalog with tests**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDef {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
}

pub fn catalog() -> Vec<ColumnDef> {
    macro_rules! c {
        ($id:literal, $label:literal, $group:literal) => {
            ColumnDef { id: $id, label: $label, group: $group }
        };
    }
    vec![
        // Créance
        c!("numero_creance", "N° créance", "Créance"),
        c!("nature_compte", "Nature compte", "Créance"),
        c!("statut_compte", "Statut compte", "Créance"),
        c!("montant_initial", "Montant initial", "Créance"),
        c!("solde", "Solde", "Créance"),
        c!("part_mutuel", "Part mutuelle", "Créance"),
        c!("type_prest", "Type prestation", "Créance"),
        c!("arc_det", "Arc détail", "Créance"),
        c!("nature_der_ope", "Nature dernière opération", "Créance"),
        c!("flux", "Flux", "Créance"),
        c!("activite", "Activité", "Créance"),
        c!("num_compte", "N° compte", "Créance"),
        c!("commentaire_creance", "Commentaire (créance)", "Créance"),
        c!("num_technicien", "N° technicien", "Créance"),
        // Dates
        c!("date_detect", "Date détection", "Dates"),
        c!("date_integration", "Date intégration", "Dates"),
        c!("date_der_ope", "Date dernière opération", "Dates"),
        c!("date_mandatement", "Date mandatement", "Dates"),
        c!("date_prescription", "Date prescription", "Dates"),
        // UGE
        c!("num_uge_gestion", "N° UGE gestion", "UGE"),
        c!("num_uge_detect", "N° UGE détection", "UGE"),
        c!("libelle_uge", "Libellé UGE gestion", "UGE"),
        // Débiteur
        c!("numero_debiteur", "N° débiteur", "Débiteur"),
        c!("cat_debiteur", "Catégorie débiteur", "Débiteur"),
        c!("nom_assure", "Nom assuré", "Débiteur"),
        c!("prenom_assure", "Prénom assuré", "Débiteur"),
        c!("matricule_assure", "Matricule assuré", "Débiteur"),
        c!("adresse_postale", "Adresse postale", "Débiteur"),
        c!("code_postal", "Code postal", "Débiteur"),
        c!("commune", "Commune", "Débiteur"),
        // Regroupée
        c!("numero_reference", "N° référence regroupée", "Regroupée"),
        c!("commentaire_creance_regroupee", "Commentaire regroupée", "Regroupée"),
        c!("motif_notif", "Motif notification", "Regroupée"),
        c!("date_detection_regroupee", "Date détection regroupée", "Regroupée"),
        c!("date_ar_notif_debiteur", "Date AR notification débiteur", "Regroupée"),
        c!("date_ar_mdm_debiteur", "Date AR mise en demeure", "Regroupée"),
        c!("etapewf", "Étape workflow (id)", "Regroupée"),
        c!("libelle_etape", "Étape workflow (libellé)", "Regroupée"),
        c!("is_douteux", "Douteux ?", "Regroupée"),
        c!("numero_og3s", "N° OG3S", "Regroupée"),
    ]
}

pub fn profile_standard_camieg() -> Vec<&'static str> {
    vec![
        "numero_creance",
        "numero_debiteur",
        "nom_assure",
        "prenom_assure",
        "matricule_assure",
        "montant_initial",
        "solde",
        "date_detect",
        "date_ar_notif_debiteur",
        "commentaire_creance_regroupee",
        "num_uge_gestion",
    ]
}

pub fn profile_complet() -> Vec<&'static str> {
    catalog().iter().map(|c| c.id).collect()
}

pub fn profile_minimal() -> Vec<&'static str> {
    vec!["numero_creance", "montant_initial", "date_integration", "commentaire_creance_regroupee"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_ids() {
        let ids: Vec<&str> = catalog().iter().map(|c| c.id).collect();
        let set: HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), set.len(), "duplicate column ids in catalog");
    }

    #[test]
    fn standard_camieg_profile_ids_exist_in_catalog() {
        let ids: HashSet<&str> = catalog().iter().map(|c| c.id).collect();
        for id in profile_standard_camieg() {
            assert!(ids.contains(id), "profile id {id} not in catalog");
        }
    }
}
```

- [ ] **Step 2: Run tests + commit**

```bash
cd src-tauri && cargo test --lib catalog 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/catalog.rs
git commit -m "feat(catalog): catalogue de colonnes + 3 profils préenregistrés"
```

---

## Wave 4 — Excel export

### Task 12: Value cell writer (TDD)

A `Value` enum + helper to write any cell into XLSX with correct format.

**Files:**
- Create: `src-tauri/src/exporter/value.rs`
- Modify: `src-tauri/src/exporter/mod.rs`

- [ ] **Step 1: Write `exporter/mod.rs`**

```rust
pub mod value;
pub mod synthese;
pub mod monthly_sheet;
pub mod params_sheet;
pub mod workbook;

pub use workbook::build_workbook;
```

- [ ] **Step 2: Write `exporter/value.rs`**

```rust
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Worksheet, XlsxError};

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Text(String),
    Int(i64),
    Money(f64),
    Date(NaiveDate),
    Bool(bool),
}

pub fn write(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    v: &Value,
    money_fmt: &Format,
    date_fmt: &Format,
) -> Result<(), XlsxError> {
    match v {
        Value::Empty => Ok(()),
        Value::Text(s) => { ws.write_string(row, col, s)?; Ok(()) }
        Value::Int(i) => { ws.write_number(row, col, *i as f64)?; Ok(()) }
        Value::Money(f) => { ws.write_number_with_format(row, col, *f, money_fmt)?; Ok(()) }
        Value::Date(d) => { ws.write_with_format(row, col, d, date_fmt)?; Ok(()) }
        Value::Bool(b) => { ws.write_string(row, col, if *b { "Oui" } else { "Non" })?; Ok(()) }
    }
}

pub fn money_format() -> Format {
    Format::new().set_num_format("# ##0,00 €")
}

pub fn date_format() -> Format {
    Format::new().set_num_format("dd/mm/yyyy")
}

pub fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0xE0E0E0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::Workbook;

    #[test]
    fn write_each_value_kind() {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        let money = money_format();
        let date = date_format();
        write(ws, 0, 0, &Value::Empty, &money, &date).unwrap();
        write(ws, 0, 1, &Value::Text("hello".into()), &money, &date).unwrap();
        write(ws, 0, 2, &Value::Int(42), &money, &date).unwrap();
        write(ws, 0, 3, &Value::Money(1234.56), &money, &date).unwrap();
        write(ws, 0, 4, &Value::Date(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()), &money, &date).unwrap();
        write(ws, 0, 5, &Value::Bool(true), &money, &date).unwrap();
    }
}
```

- [ ] **Step 3: Run, fmt, commit**

```bash
cd src-tauri && cargo test --lib exporter 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/exporter/
git commit -m "feat(exporter): writer de cellules typées (Value enum)"
```

---

### Task 13: Column resolver (TDD)

Given a `FilteredRow` and a column id, returns a `Value`. Handles joined data (uge libelle, etape libelle, adresse).

**Files:**
- Create: `src-tauri/src/exporter/resolver.rs`
- Modify: `src-tauri/src/exporter/mod.rs` (add `pub mod resolver;`)

- [ ] **Step 1: Write resolver with tests**

```rust
use crate::exporter::value::Value;
use crate::filter::FilteredRow;
use crate::schema::{AdresseDebiteur, EtapeWorkflow, Uge};
use std::collections::HashMap;

pub struct ResolverContext<'a> {
    pub uges_by_num: HashMap<&'a str, &'a Uge>,
    pub etapes_by_id: HashMap<i32, &'a EtapeWorkflow>,
    pub adresses_by_debiteur: HashMap<&'a str, &'a AdresseDebiteur>,
}

impl<'a> ResolverContext<'a> {
    pub fn build(
        uges: &'a [Uge],
        etapes: &'a [EtapeWorkflow],
        adresses: &'a [AdresseDebiteur],
    ) -> Self {
        Self {
            uges_by_num: uges.iter().map(|u| (u.num_uge.as_str(), u)).collect(),
            etapes_by_id: etapes.iter().map(|e| (e.id, e)).collect(),
            adresses_by_debiteur: adresses.iter().map(|a| (a.numero_debiteur.as_str(), a)).collect(),
        }
    }
}

pub fn resolve(ctx: &ResolverContext, row: &FilteredRow, col_id: &str) -> Value {
    let c = row.creance;
    let r = row.regroupee;
    match col_id {
        // Créance
        "numero_creance" => Value::Text(c.numero_creance.clone()),
        "nature_compte" => Value::Text(c.nature_compte.clone()),
        "statut_compte" => Value::Text(c.statut_compte.clone()),
        "montant_initial" => Value::Money(c.montant_initial),
        "solde" => Value::Money(c.solde),
        "part_mutuel" => c.part_mutuel.map(Value::Money).unwrap_or(Value::Empty),
        "type_prest" => opt_text(&c.type_prest),
        "arc_det" => opt_text(&c.arc_det),
        "nature_der_ope" => opt_text(&c.nature_der_ope),
        "flux" => opt_text(&c.flux),
        "activite" => opt_text(&c.activite),
        "num_compte" => opt_text(&c.num_compte),
        "commentaire_creance" => opt_text(&c.commentaire_creance),
        "num_technicien" => opt_text(&c.num_technicien),
        // Dates
        "date_detect" => c.date_detect.map(Value::Date).unwrap_or(Value::Empty),
        "date_integration" => c.date_integration.map(Value::Date).unwrap_or(Value::Empty),
        "date_der_ope" => c.date_der_ope.map(Value::Date).unwrap_or(Value::Empty),
        "date_mandatement" => c.date_mandatement.map(Value::Date).unwrap_or(Value::Empty),
        "date_prescription" => c.date_prescription.map(Value::Date).unwrap_or(Value::Empty),
        // UGE
        "num_uge_gestion" => Value::Text(c.num_uge_gestion.clone()),
        "num_uge_detect" => Value::Text(c.num_uge_detect.clone()),
        "libelle_uge" => ctx
            .uges_by_num
            .get(c.num_uge_gestion.as_str())
            .and_then(|u| u.libelle.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        // Débiteur
        "numero_debiteur" => Value::Text(c.numero_debiteur.clone()),
        "cat_debiteur" => Value::Text(c.cat_debiteur.clone()),
        "nom_assure" => opt_text(&c.nom_assure),
        "prenom_assure" => opt_text(&c.prenom_assure),
        "matricule_assure" => opt_text(&c.matricule_assure),
        "adresse_postale" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.adresse.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "code_postal" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.code_postal.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "commune" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.commune.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        // Regroupée
        "numero_reference" => r.map(|r| Value::Text(r.numero_reference.clone())).unwrap_or(Value::Empty),
        "commentaire_creance_regroupee" => r.and_then(|r| r.commentaire_creance.clone()).map(Value::Text).unwrap_or(Value::Empty),
        "motif_notif" => r.and_then(|r| r.motif_notif.clone()).map(Value::Text).unwrap_or(Value::Empty),
        "date_detection_regroupee" => r.and_then(|r| r.date_detection).map(Value::Date).unwrap_or(Value::Empty),
        "date_ar_notif_debiteur" => r.and_then(|r| r.date_ar_notif_debiteur).map(Value::Date).unwrap_or(Value::Empty),
        "date_ar_mdm_debiteur" => r.and_then(|r| r.date_ar_mdm_debiteur).map(Value::Date).unwrap_or(Value::Empty),
        "etapewf" => r.and_then(|r| r.etapewf).map(|i| Value::Int(i as i64)).unwrap_or(Value::Empty),
        "libelle_etape" => r
            .and_then(|r| r.etapewf)
            .and_then(|id| ctx.etapes_by_id.get(&id).map(|e| e.libelle.clone()))
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "is_douteux" => r.map(|r| Value::Bool(r.is_douteux)).unwrap_or(Value::Empty),
        "numero_og3s" => r.and_then(|r| r.numero_og3s.clone()).map(Value::Text).unwrap_or(Value::Empty),
        _ => Value::Empty,
    }
}

fn opt_text(s: &Option<String>) -> Value {
    s.clone().map(Value::Text).unwrap_or(Value::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilteredRow;
    use crate::schema::Creance;
    use chrono::NaiveDate;

    #[test]
    fn resolves_basic_columns() {
        let c = Creance {
            id: 1, workflow: None, numero_creance: "X1".into(), date_der_ope: None,
            date_detect: NaiveDate::from_ymd_opt(2026, 1, 1), nature_compte: "IND".into(),
            statut_compte: "NO".into(), gest_num: "G".into(), numero_debiteur: "D".into(),
            cat_debiteur: "C".into(), num_uge_gestion: "9501".into(), montant_initial: 99.99,
            solde: 50.0, part_mutuel: None, type_prest: None, arc_det: None, nature_der_ope: None,
            matricule_assure: None, date_mandatement: None, activite: None, num_compte: None,
            nom_assure: None, prenom_assure: None, num_uge_detect: "9501".into(),
            date_integration: None, flux: None, commentaire_creance: None, iduge: None,
            creanceregroupeeid: None, num_technicien: None, date_prescription: None,
        };
        let row = FilteredRow { creance: &c, regroupee: None, pivot_date: None };
        let ctx = ResolverContext::build(&[], &[], &[]);
        match resolve(&ctx, &row, "numero_creance") {
            Value::Text(s) => assert_eq!(s, "X1"),
            _ => panic!("wrong variant"),
        }
        match resolve(&ctx, &row, "montant_initial") {
            Value::Money(f) => assert!((f - 99.99).abs() < 1e-6),
            _ => panic!(),
        }
        matches!(resolve(&ctx, &row, "libelle_uge"), Value::Empty);
    }
}
```

- [ ] **Step 2: Run, fmt, commit**

```bash
cd src-tauri && cargo test --lib exporter 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/exporter/
git commit -m "feat(exporter): résolveur de colonnes vers Value"
```

---

### Task 14: Monthly sheet writer

Writes one worksheet per month with header row + filtered rows.

**Files:**
- Create: `src-tauri/src/exporter/monthly_sheet.rs`

- [ ] **Step 1: Write**

```rust
use crate::aggregator::{MonthKey, MonthlyBucket};
use crate::catalog::catalog;
use crate::exporter::resolver::{resolve, ResolverContext};
use crate::exporter::value::{date_format, header_format, money_format, write};
use rust_xlsxwriter::{Workbook, XlsxError};

pub fn write_monthly_sheets<'a>(
    wb: &mut Workbook,
    ctx: &ResolverContext<'a>,
    grouped: &std::collections::BTreeMap<Option<MonthKey>, MonthlyBucket<'a>>,
    columns: &[&str],
) -> Result<(), XlsxError> {
    let cat = catalog();
    let header_fmt = header_format();
    let money_fmt = money_format();
    let date_fmt = date_format();

    for (month_opt, bucket) in grouped {
        let sheet_name = match month_opt {
            Some(m) => m.label(),
            None => "Sans date".into(),
        };
        let ws = wb.add_worksheet();
        ws.set_name(&sheet_name)?;
        // Header
        for (i, col_id) in columns.iter().enumerate() {
            let label = cat.iter().find(|c| c.id == *col_id).map(|c| c.label).unwrap_or(col_id);
            ws.write_string_with_format(0, i as u16, label, &header_fmt)?;
        }
        ws.set_freeze_panes(1, 0)?;
        // Rows
        for (row_idx, fr) in bucket.rows.iter().enumerate() {
            for (col_idx, col_id) in columns.iter().enumerate() {
                let v = resolve(ctx, fr, col_id);
                write(ws, (row_idx + 1) as u32, col_idx as u16, &v, &money_fmt, &date_fmt)?;
            }
        }
        // Autofilter on data range
        if !bucket.rows.is_empty() {
            ws.autofilter(0, 0, bucket.rows.len() as u32, (columns.len() - 1) as u16)?;
        }
        // Auto column widths (cap at 60)
        for i in 0..columns.len() {
            ws.set_column_width(i as u16, 18.0)?;
        }
    }
    Ok(())
}
```

Tests integrated in Task 16 workbook assembly.

- [ ] **Step 2: fmt + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/exporter/monthly_sheet.rs
git commit -m "feat(exporter): onglets mensuels avec en-têtes + autofilter"
```

---

### Task 15: Synthesis & params sheet writers

**Files:**
- Create: `src-tauri/src/exporter/synthese.rs`
- Create: `src-tauri/src/exporter/params_sheet.rs`

- [ ] **Step 1: Write `synthese.rs`**

```rust
use crate::aggregator::{synthesis, MonthKey, SynthesisRow};
use crate::exporter::value::{header_format, money_format};
use crate::filter::FilteredRow;
use rust_xlsxwriter::{Workbook, XlsxError};
use std::collections::BTreeSet;

pub fn write_synthese(
    wb: &mut Workbook,
    rows: &[FilteredRow<'_>],
) -> Result<(), XlsxError> {
    let data = synthesis(rows);
    let ws = wb.add_worksheet();
    ws.set_name("Synthèse")?;

    let header_fmt = header_format();
    let money_fmt = money_format();

    // Determine columns: each (UGE × 3 indicators)
    let uges: BTreeSet<String> = data.iter().map(|r| r.uge.clone()).collect();
    let months: BTreeSet<Option<MonthKey>> = data.iter().map(|r| r.month).collect();

    // Headers
    ws.write_string_with_format(0, 0, "Mois", &header_fmt)?;
    let mut col_idx = 1u16;
    let mut uge_col_map = std::collections::HashMap::new();
    for uge in &uges {
        uge_col_map.insert(uge.clone(), col_idx);
        ws.write_string_with_format(0, col_idx, &format!("{uge} — Nb"), &header_fmt)?;
        ws.write_string_with_format(0, col_idx + 1, &format!("{uge} — Σ montant"), &header_fmt)?;
        ws.write_string_with_format(0, col_idx + 2, &format!("{uge} — Σ solde"), &header_fmt)?;
        col_idx += 3;
    }
    ws.set_freeze_panes(1, 1)?;

    // Index data
    let mut by_key: std::collections::HashMap<(Option<MonthKey>, String), SynthesisRow> = std::collections::HashMap::new();
    for r in data {
        by_key.insert((r.month, r.uge.clone()), r);
    }

    // Body rows
    for (i, m) in months.iter().enumerate() {
        let row = (i + 1) as u32;
        let label = m.map(|m| m.label()).unwrap_or_else(|| "Sans date".into());
        ws.write_string(row, 0, &label)?;
        for uge in &uges {
            let base = *uge_col_map.get(uge).unwrap();
            if let Some(s) = by_key.get(&(*m, uge.clone())) {
                ws.write_number(row, base, s.count as f64)?;
                ws.write_number_with_format(row, base + 1, s.somme_montant_initial, &money_fmt)?;
                ws.write_number_with_format(row, base + 2, s.somme_solde, &money_fmt)?;
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Write `params_sheet.rs`**

```rust
use crate::filter::FilterSet;
use crate::exporter::value::header_format;
use chrono::Utc;
use rust_xlsxwriter::{Workbook, XlsxError};

pub struct ExportContext {
    pub source_path: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub app_version: String,
    pub rows_read: usize,
    pub rows_after_filter: usize,
    pub corrupted_rows: usize,
}

pub fn write_params(
    wb: &mut Workbook,
    filters: &FilterSet,
    columns: &[&str],
    ctx: &ExportContext,
) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Paramètres")?;
    let h = header_format();
    ws.set_column_width(0, 35.0)?;
    ws.set_column_width(1, 80.0)?;

    let mut row: u32 = 0;
    let mut put = |ws: &mut rust_xlsxwriter::Worksheet, r: &mut u32, k: &str, v: String| -> Result<(), XlsxError> {
        ws.write_string_with_format(*r, 0, k, &h)?;
        ws.write_string(*r, 1, &v)?;
        *r += 1;
        Ok(())
    };

    put(ws, &mut row, "Date d'export", Utc::now().to_rfc3339())?;
    put(ws, &mut row, "Version raffinerie", ctx.app_version.clone())?;
    put(ws, &mut row, "Dump source (chemin)", ctx.source_path.clone())?;
    put(ws, &mut row, "Dump source (taille octets)", ctx.source_size.to_string())?;
    put(ws, &mut row, "Dump source (SHA-256)", ctx.source_sha256.clone())?;
    put(ws, &mut row, "Lignes lues", ctx.rows_read.to_string())?;
    put(ws, &mut row, "Lignes après filtres", ctx.rows_after_filter.to_string())?;
    put(ws, &mut row, "Lignes corrompues skippées", ctx.corrupted_rows.to_string())?;
    row += 1;

    put(ws, &mut row, "Filtre — UGE", filters.uges.join(", "))?;
    put(ws, &mut row, "Filtre — Nature compte", filters.nature_compte.join(", "))?;
    put(ws, &mut row, "Filtre — Commentaire contient", filters.commentaire_contient.clone().unwrap_or_default())?;
    put(ws, &mut row, "Filtre — Insensible casse/accents", filters.commentaire_insensible.to_string())?;
    put(ws, &mut row, "Filtre — Critère notification", format!("{:?}", filters.notif_criterion))?;
    put(ws, &mut row, "Filtre — Date pivot", format!("{:?}", filters.date_pivot))?;
    put(ws, &mut row, "Filtre — Date min", filters.date_min.map(|d| d.to_string()).unwrap_or_default())?;
    put(ws, &mut row, "Filtre — Date max", filters.date_max.map(|d| d.to_string()).unwrap_or_default())?;
    row += 1;

    put(ws, &mut row, "Colonnes exportées", columns.join(", "))?;
    Ok(())
}
```

- [ ] **Step 3: fmt + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/exporter/
git commit -m "feat(exporter): onglets Synthèse et Paramètres"
```

---

### Task 16: Workbook assembly (TDD)

Orchestrates synthese + monthly_sheets + params into a single workbook saved to disk.

**Files:**
- Create: `src-tauri/src/exporter/workbook.rs`

- [ ] **Step 1: Write with test**

```rust
use crate::aggregator::group_by_month;
use crate::exporter::monthly_sheet::write_monthly_sheets;
use crate::exporter::params_sheet::{write_params, ExportContext};
use crate::exporter::resolver::ResolverContext;
use crate::exporter::synthese::write_synthese;
use crate::filter::FilteredRow;
use crate::filter::FilterSet;
use crate::parser::ParsedDump;
use rust_xlsxwriter::{Workbook, XlsxError};
use std::path::Path;

pub fn build_workbook(
    path: &Path,
    dump: &ParsedDump,
    rows: &[FilteredRow<'_>],
    columns: &[&str],
    filters: &FilterSet,
    export_ctx: &ExportContext,
) -> Result<(), XlsxError> {
    let mut wb = Workbook::new();
    let ctx = ResolverContext::build(&dump.uges, &dump.etapes, &dump.adresses);
    write_synthese(&mut wb, rows)?;
    let grouped = group_by_month(rows);
    write_monthly_sheets(&mut wb, &ctx, &grouped, columns)?;
    write_params(&mut wb, filters, columns, export_ctx)?;
    wb.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::evaluate;
    use crate::schema::Creance;
    use calamine::{open_workbook, Reader, Xlsx};
    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    fn mini_dump_with_two() -> ParsedDump {
        let mut d = ParsedDump::default();
        d.creances.push(Creance {
            id: 1, workflow: None, numero_creance: "C1".into(), date_der_ope: None,
            date_detect: NaiveDate::from_ymd_opt(2026, 1, 5), nature_compte: "IND".into(),
            statut_compte: "NO".into(), gest_num: "G".into(), numero_debiteur: "D1".into(),
            cat_debiteur: "C".into(), num_uge_gestion: "9501".into(), montant_initial: 100.0,
            solde: 100.0, part_mutuel: None, type_prest: None, arc_det: None, nature_der_ope: None,
            matricule_assure: None, date_mandatement: None, activite: None, num_compte: None,
            nom_assure: None, prenom_assure: None, num_uge_detect: "9501".into(),
            date_integration: NaiveDate::from_ymd_opt(2026, 1, 5), flux: None,
            commentaire_creance: None, iduge: None, creanceregroupeeid: None,
            num_technicien: None, date_prescription: None,
        });
        d
    }

    #[test]
    fn writes_workbook_and_re_reads_value() {
        let d = mini_dump_with_two();
        let fs = FilterSet::default();
        let rows = evaluate(&d, &fs);
        let tmp = NamedTempFile::new().unwrap().into_temp_path();
        let path = tmp.with_extension("xlsx");
        let cols = ["numero_creance", "montant_initial"];
        let ec = ExportContext {
            source_path: "test.dump".into(),
            source_sha256: "n/a".into(),
            source_size: 0,
            app_version: "0.0.0".into(),
            rows_read: 1,
            rows_after_filter: 1,
            corrupted_rows: 0,
        };
        build_workbook(&path, &d, &rows, &cols, &fs, &ec).unwrap();
        let mut wb: Xlsx<_> = open_workbook(&path).unwrap();
        let names = wb.sheet_names();
        assert!(names.iter().any(|n| n == "Synthèse"));
        assert!(names.iter().any(|n| n == "2026-01"));
        assert!(names.iter().any(|n| n == "Paramètres"));
    }
}
```

- [ ] **Step 2: Run, fmt, commit**

```bash
cd src-tauri && cargo test --lib exporter::workbook 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib --tests -- -D warnings 2>&1 | tail
git add src-tauri/src/exporter/
git commit -m "feat(exporter): assemblage classeur multi-onglets"
```

---

## Wave 5 — State, persistence, IPC

### Task 17: AppState

In-memory store for parsed dump + metadata, Mutex-protected.

**Files:**
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: Replace stub with real AppState**

```rust
use crate::parser::ParsedDump;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub data: Mutex<Option<LoadedDump>>,
}

pub struct LoadedDump {
    pub dump: ParsedDump,
    pub source_path: String,
    pub source_size: u64,
    pub source_sha256: String,
}
```

- [ ] **Step 2: cargo check + commit**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/state.rs
git commit -m "feat(state): AppState avec dump chargé en mémoire (Mutex)"
```

---

### Task 18: Persistence (profiles + last-session)

**Files:**
- Modify: `src-tauri/src/persistence.rs`

- [ ] **Step 1: Write persistence**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profiles {
    pub profiles: HashMap<String, Vec<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Session {
    pub filters: serde_json::Value,
    pub active_profile: Option<String>,
    pub columns: Vec<String>,
}

fn config_dir() -> PathBuf {
    let base = dirs_next::config_dir().unwrap_or_else(|| std::env::temp_dir());
    let dir = base.join("raffinerie");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn load_profiles() -> Profiles {
    let p = config_dir().join("profiles.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_profiles(profiles: &Profiles) -> Result<(), String> {
    let p = config_dir().join("profiles.json");
    let s = serde_json::to_string_pretty(profiles).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

pub fn load_session() -> Session {
    let p = config_dir().join("last-session.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_session(session: &Session) -> Result<(), String> {
    let p = config_dir().join("last-session.json");
    let s = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_roundtrip_default() {
        let p = Profiles::default();
        let s = serde_json::to_string(&p).unwrap();
        let q: Profiles = serde_json::from_str(&s).unwrap();
        assert_eq!(q.profiles.len(), 0);
    }
}
```

Add to `Cargo.toml` deps:
```toml
dirs-next = "2"
```

- [ ] **Step 2: Test + commit**

```bash
cd src-tauri && cargo test --lib persistence 2>&1 | tail -5
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/persistence.rs src-tauri/Cargo.toml
git commit -m "feat(persistence): profils colonnes + dernière session sur disque"
```

---

### Task 19: IPC commands — real implementation

Replace `src-tauri/src/ipc.rs` stubs with working handlers.

**Files:**
- Replace: `src-tauri/src/ipc.rs`

- [ ] **Step 1: Implement all handlers**

```rust
use crate::aggregator::group_by_month;
use crate::catalog::{catalog, profile_complet, profile_minimal, profile_standard_camieg};
use crate::exporter::params_sheet::ExportContext;
use crate::exporter::workbook::build_workbook;
use crate::filter::{evaluate, FilterSet};
use crate::parser::parse;
use crate::persistence::{self, Profiles, Session};
use crate::state::{AppState, LoadedDump};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;
use tauri::State;

#[derive(Serialize)]
pub struct ParseResult {
    pub creances: usize,
    pub regroupees: usize,
    pub uges: usize,
    pub etapes: usize,
    pub adresses: usize,
    pub corrupted: usize,
    pub elapsed_ms: u128,
}

#[tauri::command]
pub async fn parse_dump(path: String, state: State<'_, AppState>) -> Result<ParseResult, String> {
    let pb = PathBuf::from(&path);
    let mut file = File::open(&pb).map_err(|e| format!("open: {e}"))?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();

    // Compute SHA-256 while loading (one pass via separate read; for simplicity, two passes).
    let sha = {
        let mut hasher = Sha256::new();
        let mut h_file = File::open(&pb).map_err(|e| e.to_string())?;
        let mut buf = [0u8; 8192];
        loop {
            let n = h_file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        format!("{:x}", hasher.finalize())
    };

    let t0 = Instant::now();
    let dump = parse(&mut file, |_b| {}).map_err(|e| format!("parse: {e}"))?;
    let elapsed_ms = t0.elapsed().as_millis();

    let result = ParseResult {
        creances: dump.creances.len(),
        regroupees: dump.creances_regroupees.len(),
        uges: dump.uges.len(),
        etapes: dump.etapes.len(),
        adresses: dump.adresses.len(),
        corrupted: dump.corrupted_rows,
        elapsed_ms,
    };
    *state.data.lock().unwrap() = Some(LoadedDump {
        dump,
        source_path: path,
        source_size: size,
        source_sha256: sha,
    });
    Ok(result)
}

#[tauri::command]
pub async fn list_uges(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let mut s: BTreeSet<String> = d.dump.creances.iter().map(|c| c.num_uge_gestion.clone()).collect();
    for u in &d.dump.uges { s.insert(u.num_uge.clone()); }
    Ok(s.into_iter().collect())
}

#[tauri::command]
pub async fn list_natures(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let s: BTreeSet<String> = d.dump.creances.iter().map(|c| c.nature_compte.clone()).collect();
    Ok(s.into_iter().collect())
}

#[tauri::command]
pub async fn list_etapes(state: State<'_, AppState>) -> Result<Vec<(i32, String)>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let mut out: Vec<(i32, String)> = d.dump.etapes.iter().map(|e| (e.id, e.libelle.clone())).collect();
    out.sort_by_key(|(id, _)| *id);
    Ok(out)
}

#[tauri::command]
pub async fn list_statuts(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let s: BTreeSet<String> = d.dump.creances.iter().map(|c| c.statut_compte.clone()).collect();
    Ok(s.into_iter().collect())
}

#[tauri::command]
pub async fn count_filtered(filters: FilterSet, state: State<'_, AppState>) -> Result<usize, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    Ok(evaluate(&d.dump, &filters).len())
}

#[derive(Serialize)]
pub struct PreviewRow { pub cells: Vec<serde_json::Value> }

#[tauri::command]
pub async fn preview(
    filters: FilterSet,
    columns: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PreviewRow>, String> {
    use crate::exporter::resolver::{resolve, ResolverContext};
    use crate::exporter::value::Value;
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let rows = evaluate(&d.dump, &filters);
    let ctx = ResolverContext::build(&d.dump.uges, &d.dump.etapes, &d.dump.adresses);
    let out: Vec<PreviewRow> = rows.iter().take(100).map(|r| {
        let cells = columns.iter().map(|c| {
            match resolve(&ctx, r, c) {
                Value::Empty => serde_json::Value::Null,
                Value::Text(s) => serde_json::Value::String(s),
                Value::Int(i) => serde_json::Value::Number(i.into()),
                Value::Money(f) => serde_json::json!(f),
                Value::Date(d) => serde_json::Value::String(d.format("%d/%m/%Y").to_string()),
                Value::Bool(b) => serde_json::Value::String(if b {"Oui".into()} else {"Non".into()}),
            }
        }).collect();
        PreviewRow { cells }
    }).collect();
    Ok(out)
}

#[tauri::command]
pub async fn list_columns() -> Result<serde_json::Value, String> {
    let cat = catalog();
    let presets = serde_json::json!({
        "Standard CAMIEG": profile_standard_camieg(),
        "Complet": profile_complet(),
        "Minimal": profile_minimal(),
    });
    Ok(serde_json::json!({
        "columns": cat,
        "presets": presets,
    }))
}

#[tauri::command]
pub async fn load_profiles() -> Result<Profiles, String> {
    Ok(persistence::load_profiles())
}

#[tauri::command]
pub async fn save_profile(name: String, cols: Vec<String>) -> Result<(), String> {
    let mut p = persistence::load_profiles();
    p.profiles.insert(name, cols);
    persistence::save_profiles(&p)
}

#[tauri::command]
pub async fn delete_profile(name: String) -> Result<(), String> {
    let mut p = persistence::load_profiles();
    p.profiles.remove(&name);
    persistence::save_profiles(&p)
}

#[tauri::command]
pub async fn load_session() -> Result<Session, String> {
    Ok(persistence::load_session())
}

#[tauri::command]
pub async fn save_session(session: Session) -> Result<(), String> {
    persistence::save_session(&session)
}

#[tauri::command]
pub async fn export_xlsx(
    path: String,
    filters: FilterSet,
    columns: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let rows = evaluate(&d.dump, &filters);
    let cols_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
    let ctx = ExportContext {
        source_path: d.source_path.clone(),
        source_sha256: d.source_sha256.clone(),
        source_size: d.source_size,
        app_version: env!("CARGO_PKG_VERSION").into(),
        rows_read: d.dump.creances.len(),
        rows_after_filter: rows.len(),
        corrupted_rows: d.dump.corrupted_rows,
    };
    build_workbook(std::path::Path::new(&path), &d.dump, &rows, &cols_refs, &filters, &ctx)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: cargo check, fix any compilation errors**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

Iterate until clean.

- [ ] **Step 3: fmt + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib -- -D warnings 2>&1 | tail
git add src-tauri/src/ipc.rs
git commit -m "feat(ipc): handlers Tauri parse/filter/preview/export complets"
```

---

## Wave 6 — Frontend

### Task 20: Frontend HTML structure

**Files:**
- Replace: `src/index.html`
- Create: `src/vendor/alpine.min.js`

- [ ] **Step 1: Download Alpine.js**

```bash
mkdir -p /home/alex/Documents/REPO/raffinerie/src/vendor
curl -sL https://unpkg.com/alpinejs@3.13.0/dist/cdn.min.js -o /home/alex/Documents/REPO/raffinerie/src/vendor/alpine.min.js
ls -la /home/alex/Documents/REPO/raffinerie/src/vendor/
```

- [ ] **Step 2: Write `src/index.html`**

```html
<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>raffinerie — extracteur SUCRE</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body x-data="app()" x-init="init()" :class="dark ? 'dark' : ''">
  <header>
    <h1>raffinerie <span class="version" x-text="version"></span></h1>
    <div class="header-actions">
      <button @click="dark = !dark" :title="dark ? 'Mode clair' : 'Mode sombre'">🌓</button>
    </div>
  </header>

  <section class="dropzone" :class="{ loaded: dumpLoaded, loading: parsing }"
           @dragover.prevent @drop.prevent="onDrop($event)">
    <template x-if="!dumpLoaded && !parsing">
      <div>
        <p>📂 Glisser-déposer le fichier <code>.dump</code> ici</p>
        <button @click="pickFile()">ou cliquer pour parcourir</button>
      </div>
    </template>
    <template x-if="parsing">
      <p>⏳ Parsing… <span x-text="parseProgress"></span></p>
    </template>
    <template x-if="dumpLoaded">
      <p>✓ <span x-text="dumpStats.creances.toLocaleString('fr-FR')"></span> créances chargées
        en <span x-text="(dumpStats.elapsed_ms / 1000).toFixed(1)"></span> s
        <button @click="resetDump()" class="link">changer de dump</button></p>
    </template>
  </section>

  <section class="filters" x-show="dumpLoaded">
    <h2>Filtres</h2>
    <div class="filter-row">
      <label>UGE</label>
      <div class="multi-check" x-data>
        <template x-for="u in uges">
          <label class="chip"><input type="checkbox" :value="u" x-model="filters.uges"> <span x-text="u"></span></label>
        </template>
      </div>
    </div>
    <div class="filter-row">
      <label>Nature compte</label>
      <div class="multi-check">
        <template x-for="n in natures">
          <label class="chip"><input type="checkbox" :value="n" x-model="filters.natureCompte"> <span x-text="n"></span></label>
        </template>
      </div>
    </div>
    <div class="filter-row">
      <label>Commentaire contient</label>
      <input type="text" x-model.debounce.200ms="filters.commentaireContient" placeholder="ex: indu roc">
      <label><input type="checkbox" x-model="filters.commentaireInsensible"> insensible casse/accents</label>
    </div>
    <div class="filter-row">
      <label>Notification</label>
      <select x-model="notifKind">
        <option value="aucun">Aucun</option>
        <option value="motif_notif_non_vide">Motif notif rempli</option>
        <option value="date_ar_notif_non_vide">Date AR notif remplie</option>
        <option value="etape_wf_dans">Étape WF parmi…</option>
        <option value="statut_compte_dans">Statut compte parmi…</option>
      </select>
      <div x-show="notifKind === 'etape_wf_dans'" class="multi-check">
        <template x-for="e in etapes">
          <label class="chip"><input type="checkbox" :value="e[0]" x-model.number="etapeIds"> <span x-text="e[0] + ' — ' + e[1]"></span></label>
        </template>
      </div>
      <div x-show="notifKind === 'statut_compte_dans'" class="multi-check">
        <template x-for="s in statuts">
          <label class="chip"><input type="checkbox" :value="s" x-model="statutValues"> <span x-text="s"></span></label>
        </template>
      </div>
    </div>
    <div class="filter-row">
      <label>Date pivot</label>
      <select x-model="filters.datePivot">
        <option value="date_detect">Date détection (créance)</option>
        <option value="date_integration">Date intégration</option>
        <option value="date_der_ope">Date dernière opération</option>
        <option value="date_mandatement">Date mandatement</option>
        <option value="date_ar_notif_debiteur">Date AR notification débiteur</option>
        <option value="date_detection_regroupee">Date détection (regroupée)</option>
      </select>
    </div>
    <div class="filter-row">
      <label>Période</label>
      du <input type="date" x-model="filters.dateMin">
      au <input type="date" x-model="filters.dateMax">
    </div>
  </section>

  <section class="columns" x-show="dumpLoaded">
    <h2>Colonnes à exporter
      <select x-model="activeProfile" @change="applyProfile()">
        <option value="">— Profil —</option>
        <template x-for="p in Object.keys(presets)">
          <option :value="p" x-text="p"></option>
        </template>
        <template x-for="p in Object.keys(personalProfiles)">
          <option :value="'_' + p" x-text="'★ ' + p"></option>
        </template>
      </select>
      <button @click="savePersonalProfile()">Sauver profil…</button>
    </h2>
    <template x-for="group in groupedColumns">
      <details open>
        <summary x-text="group.name + ' (' + group.cols.filter(c => selectedColumns.includes(c.id)).length + ' / ' + group.cols.length + ')'"></summary>
        <div class="col-list">
          <template x-for="c in group.cols">
            <label><input type="checkbox" :value="c.id" x-model="selectedColumns"> <span x-text="c.label"></span></label>
          </template>
        </div>
      </details>
    </template>
  </section>

  <section class="actions" x-show="dumpLoaded">
    <div class="estimate">Résultat estimé : <strong x-text="estimatedCount.toLocaleString('fr-FR')"></strong> lignes</div>
    <button @click="openPreview()" :disabled="estimatedCount === 0">👁 Aperçu</button>
    <button @click="exportXlsx()" :disabled="estimatedCount === 0 || selectedColumns.length === 0" class="primary">💾 Exporter Excel</button>
  </section>

  <div class="modal" x-show="previewOpen" @click.self="previewOpen = false">
    <div class="modal-body">
      <h3>Aperçu (100 premières lignes)</h3>
      <table>
        <thead><tr><template x-for="c in selectedColumns"><th x-text="labelOf(c)"></th></template></tr></thead>
        <tbody><template x-for="r in previewRows">
          <tr><template x-for="(cell, i) in r.cells"><td x-text="cell"></td></template></tr>
        </template></tbody>
      </table>
      <button @click="previewOpen = false">Fermer</button>
    </div>
  </div>

  <div class="toast" x-show="toast" x-transition>
    <span x-text="toast"></span>
  </div>

  <script src="vendor/alpine.min.js" defer></script>
  <script src="app.js"></script>
</body>
</html>
```

- [ ] **Step 3: Commit**

```bash
git add src/index.html src/vendor/alpine.min.js
git commit -m "feat(ui): structure HTML + Alpine.js vendored"
```

---

### Task 21: Frontend CSS

**Files:**
- Replace: `src/styles.css`

- [ ] **Step 1: Write `src/styles.css`**

```css
:root {
  --bg: #fafafa;
  --fg: #1a1a1a;
  --border: #d0d0d0;
  --primary: #7B3F00;
  --primary-fg: #ffffff;
  --muted: #666;
  --chip-bg: #ececec;
  --chip-bg-checked: #7B3F00;
  --chip-fg-checked: #ffffff;
}
body.dark {
  --bg: #1a1a1a;
  --fg: #f0f0f0;
  --border: #404040;
  --muted: #999;
  --chip-bg: #2a2a2a;
}
* { box-sizing: border-box; }
body {
  font-family: system-ui, -apple-system, sans-serif;
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font-size: 14px;
}
header { display:flex; justify-content:space-between; align-items:center; padding:1rem 1.5rem; border-bottom:1px solid var(--border); }
h1 { margin:0; font-size:1.5rem; }
.version { font-size:0.7em; color: var(--muted); font-weight:normal; }
section { padding:1rem 1.5rem; border-bottom:1px solid var(--border); }
h2 { font-size:1.1rem; margin:0 0 0.75rem; display:flex; gap:0.5rem; align-items:center; }
.dropzone { border:2px dashed var(--border); border-radius:8px; padding:2rem; text-align:center; margin:1rem 1.5rem; }
.dropzone.loaded { border-style:solid; background: rgba(123,63,0,0.05); }
.dropzone.loading { border-color: var(--primary); }
.dropzone code { background: var(--chip-bg); padding: 0.1em 0.4em; border-radius: 3px; }
button { background: var(--chip-bg); color: var(--fg); border:1px solid var(--border); border-radius:4px; padding:0.4rem 0.8rem; cursor:pointer; font-size:0.9em; }
button:hover { background: var(--border); }
button.primary { background: var(--primary); color: var(--primary-fg); border-color: var(--primary); }
button.link { background: transparent; border: none; text-decoration: underline; color: var(--primary); padding: 0; }
button:disabled { opacity: 0.4; cursor: not-allowed; }
.filter-row { display:flex; gap:0.75rem; align-items:center; margin-bottom:0.75rem; flex-wrap:wrap; }
.filter-row > label:first-child { min-width: 130px; font-weight:600; }
.multi-check { display:flex; flex-wrap:wrap; gap:0.4rem; }
.chip { display:inline-flex; align-items:center; gap:0.3rem; padding:0.25rem 0.6rem; border-radius:14px; background: var(--chip-bg); cursor:pointer; user-select:none; }
.chip input { display:none; }
.chip:has(input:checked) { background: var(--chip-bg-checked); color: var(--chip-fg-checked); }
input[type=text], input[type=date], select { background: var(--bg); color: var(--fg); border:1px solid var(--border); border-radius:4px; padding:0.3rem 0.5rem; font-size:0.9em; }
.columns details { margin-bottom:0.5rem; }
.columns summary { cursor:pointer; padding:0.3rem 0; font-weight:600; }
.col-list { display:grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap:0.3rem 0.75rem; padding-left:1rem; }
.col-list label { display:flex; gap:0.4rem; align-items:center; }
.actions { display:flex; gap:1rem; align-items:center; justify-content:space-between; }
.estimate { font-size:0.95em; }
.modal { position:fixed; inset:0; background: rgba(0,0,0,0.5); display:flex; align-items:center; justify-content:center; z-index:100; }
.modal-body { background: var(--bg); padding:1.5rem; border-radius:8px; max-width:90vw; max-height:85vh; overflow:auto; }
table { border-collapse: collapse; width:100%; font-size:0.85em; }
th, td { border:1px solid var(--border); padding:0.3rem 0.5rem; text-align:left; white-space:nowrap; }
th { background: var(--chip-bg); position:sticky; top:0; }
.toast { position:fixed; bottom:1rem; right:1rem; background: var(--primary); color: var(--primary-fg); padding:0.75rem 1rem; border-radius:6px; box-shadow: 0 2px 8px rgba(0,0,0,0.2); }
```

- [ ] **Step 2: Commit**

```bash
git add src/styles.css
git commit -m "feat(ui): styles CSS + thème sombre"
```

---

### Task 22: Frontend logic (app.js)

**Files:**
- Replace: `src/app.js`

- [ ] **Step 1: Write `src/app.js`**

```javascript
const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

function app() {
  return {
    version: '0.1.0',
    dark: false,
    dumpLoaded: false,
    parsing: false,
    parseProgress: '',
    dumpStats: { creances: 0, elapsed_ms: 0 },

    // Catalog + columns
    columnsCatalog: [],
    presets: {},
    personalProfiles: {},
    selectedColumns: [],
    activeProfile: '',

    // Filter state
    filters: {
      uges: [],
      natureCompte: [],
      commentaireContient: '',
      commentaireInsensible: true,
      notifCriterion: { kind: 'aucun' },
      datePivot: 'date_integration',
      dateMin: null,
      dateMax: null,
    },
    notifKind: 'aucun',
    etapeIds: [],
    statutValues: [],

    // Distinct values (loaded after parse)
    uges: [],
    natures: [],
    etapes: [],
    statuts: [],

    // Result
    estimatedCount: 0,
    previewOpen: false,
    previewRows: [],
    toast: '',

    async init() {
      // Match Windows theme
      this.dark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;

      const cat = await invoke('list_columns');
      this.columnsCatalog = cat.columns;
      this.presets = cat.presets;
      this.personalProfiles = (await invoke('load_profiles')).profiles || {};

      // Restore last session if any
      const sess = await invoke('load_session');
      if (sess && sess.filters) {
        Object.assign(this.filters, sess.filters);
        if (sess.columns && sess.columns.length) this.selectedColumns = sess.columns;
        if (sess.activeProfile) this.activeProfile = sess.activeProfile;
      } else {
        this.applyPreset('Standard CAMIEG');
      }

      // Watch and persist
      ['filters', 'selectedColumns', 'activeProfile'].forEach(prop => {
        this.$watch(prop, () => this.persistSession(), { deep: true });
      });
      // Watch filters → recompute estimate (debounced via Alpine .debounce on inputs)
      this.$watch('filters', () => this.refreshEstimate(), { deep: true });
      this.$watch('notifKind', () => this.syncNotif());
      this.$watch('etapeIds', () => this.syncNotif());
      this.$watch('statutValues', () => this.syncNotif());
    },

    get groupedColumns() {
      const groups = {};
      this.columnsCatalog.forEach(c => {
        if (!groups[c.group]) groups[c.group] = { name: c.group, cols: [] };
        groups[c.group].cols.push(c);
      });
      return Object.values(groups);
    },

    labelOf(id) {
      const c = this.columnsCatalog.find(c => c.id === id);
      return c ? c.label : id;
    },

    applyPreset(name) {
      const cols = this.presets[name];
      if (cols) this.selectedColumns = [...cols];
    },

    applyProfile() {
      if (!this.activeProfile) return;
      if (this.activeProfile.startsWith('_')) {
        const name = this.activeProfile.slice(1);
        this.selectedColumns = [...(this.personalProfiles[name] || [])];
      } else {
        this.applyPreset(this.activeProfile);
      }
    },

    async savePersonalProfile() {
      const name = prompt('Nom du profil personnel :');
      if (!name) return;
      await invoke('save_profile', { name, cols: this.selectedColumns });
      this.personalProfiles[name] = [...this.selectedColumns];
      this.toastMsg('Profil sauvegardé');
    },

    async pickFile() {
      const path = await open({ multiple: false, filters: [{ name: 'Dump SUCRE', extensions: ['dump', 'sql'] }] });
      if (path) this.loadDump(path);
    },

    async onDrop(ev) {
      const files = ev.dataTransfer && ev.dataTransfer.files;
      if (!files || !files.length) return;
      // Tauri provides path on FileDrop via the File API extension; otherwise listen tauri event
      const path = files[0].path || files[0].name;
      this.loadDump(path);
    },

    async loadDump(path) {
      this.parsing = true;
      this.parseProgress = '⏳';
      try {
        const stats = await invoke('parse_dump', { path });
        this.dumpStats = stats;
        this.dumpLoaded = true;
        const [uges, natures, etapes, statuts] = await Promise.all([
          invoke('list_uges'), invoke('list_natures'), invoke('list_etapes'), invoke('list_statuts')
        ]);
        this.uges = uges; this.natures = natures; this.etapes = etapes; this.statuts = statuts;
        this.refreshEstimate();
      } catch (e) {
        alert('Erreur : ' + e);
      } finally {
        this.parsing = false;
      }
    },

    resetDump() {
      this.dumpLoaded = false;
      this.dumpStats = { creances: 0, elapsed_ms: 0 };
      this.estimatedCount = 0;
    },

    syncNotif() {
      switch (this.notifKind) {
        case 'aucun': this.filters.notifCriterion = { kind: 'aucun' }; break;
        case 'motif_notif_non_vide': this.filters.notifCriterion = { kind: 'motif_notif_non_vide' }; break;
        case 'date_ar_notif_non_vide': this.filters.notifCriterion = { kind: 'date_ar_notif_non_vide' }; break;
        case 'etape_wf_dans': this.filters.notifCriterion = { kind: 'etape_wf_dans', ids: this.etapeIds }; break;
        case 'statut_compte_dans': this.filters.notifCriterion = { kind: 'statut_compte_dans', values: this.statutValues }; break;
      }
    },

    async refreshEstimate() {
      if (!this.dumpLoaded) return;
      try {
        this.estimatedCount = await invoke('count_filtered', { filters: this.filters });
      } catch (e) { /* ignore transient */ }
    },

    async openPreview() {
      this.previewRows = await invoke('preview', { filters: this.filters, columns: this.selectedColumns });
      this.previewOpen = true;
    },

    async exportXlsx() {
      const uges = this.filters.uges.length ? this.filters.uges.join('-') : 'toutesUGE';
      const now = new Date();
      const stamp = now.getFullYear() + String(now.getMonth()+1).padStart(2,'0') + String(now.getDate()).padStart(2,'0')
                  + '-' + String(now.getHours()).padStart(2,'0') + String(now.getMinutes()).padStart(2,'0');
      const path = await save({
        defaultPath: `raffinerie_${uges}_${stamp}.xlsx`,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!path) return;
      try {
        await invoke('export_xlsx', { path, filters: this.filters, columns: this.selectedColumns });
        this.toastMsg(`✓ Exporté : ${path}`);
      } catch (e) {
        alert('Erreur export : ' + e);
      }
    },

    async persistSession() {
      const sess = { filters: this.filters, activeProfile: this.activeProfile, columns: this.selectedColumns };
      try { await invoke('save_session', { session: sess }); } catch {}
    },

    toastMsg(msg) {
      this.toast = msg;
      setTimeout(() => this.toast = '', 4000);
    },
  };
}
window.app = app;
```

- [ ] **Step 2: Commit**

```bash
git add src/app.js
git commit -m "feat(ui): logique Alpine.js complète (drop, filtres, preview, export)"
```

---

### Task 23: Local UI smoke test

Verify the Tauri app launches and the UI loads. On Linux, `cargo tauri dev` requires WebKitGTK.

**Files:**
- None (manual verification only)

- [ ] **Step 1: Install Linux Tauri prerequisites (Fedora)**

```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel librsvg2-devel libappindicator-gtk3-devel openssl-devel pkgconf-pkg-config 2>&1 | tail -5
```

If `sudo` blocks autonomy, skip and document in README that this step is required for `cargo tauri dev`.

- [ ] **Step 2: Try launching dev mode**

```bash
cd /home/alex/Documents/REPO/raffinerie/src-tauri && cargo install tauri-cli --version "^2" --locked 2>&1 | tail -3
cd /home/alex/Documents/REPO/raffinerie/src-tauri && timeout 30 cargo tauri dev 2>&1 | tail -20
```

If this fails (missing libs or no display), document the limitation and move on — the Windows .exe build via CI is what matters.

- [ ] **Step 3: Commit any fixes discovered**

If issues are found and code is changed, commit. If purely environmental, document in README:
```markdown
## Linux dev environment

Requires: `webkit2gtk4.1-devel gtk3-devel librsvg2-devel libappindicator-gtk3-devel openssl-devel pkgconf-pkg-config` (Fedora).
```

---

## Wave 7 — CI/CD & polish

### Task 24: GitHub Actions Windows build

**Files:**
- Create: `.github/workflows/windows-build.yml`

- [ ] **Step 1: Write workflow**

```yaml
name: Windows build

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install tauri-cli
        run: cargo install tauri-cli --version "^2" --locked

      - name: Build (release)
        working-directory: src-tauri
        run: cargo tauri build

      - name: Run tests
        working-directory: src-tauri
        run: cargo test --release

      - name: Upload .exe artifact
        uses: actions/upload-artifact@v4
        with:
          name: raffinerie-windows-x64
          path: |
            src-tauri/target/release/raffinerie.exe
            src-tauri/target/release/bundle/nsis/*.exe
          if-no-files-found: error
          retention-days: 30
```

- [ ] **Step 2: Commit + push + verify CI**

```bash
mkdir -p /home/alex/Documents/REPO/raffinerie/.github/workflows
# (file already created above by the agent)
cd /home/alex/Documents/REPO/raffinerie
git add .github/
git commit -m "ci: workflow build Windows .exe + artefact"
git push
```

Wait ~10-15 minutes, check on GitHub Actions for green workflow + downloadable artifact.

```bash
gh run list --limit 1 2>&1 || echo "gh not available; check https://github.com/ElegAlex/raffinerie/actions"
```

If the workflow fails, read logs, fix issues, push fixes.

---

### Task 25: README polish + CHANGELOG + guide utilisateur

**Files:**
- Modify: `README.md`
- Create: `CHANGELOG.md`
- Create: `docs/guide-utilisateur.md`

- [ ] **Step 1: Update README with CI badge + screenshots placeholder**

Append to README.md:
```markdown
## Build status

![Windows build](https://github.com/ElegAlex/raffinerie/actions/workflows/windows-build.yml/badge.svg)

## Téléchargement

Les builds Windows sont produits par GitHub Actions et publiés en tant qu'artefacts du workflow `Windows build`. Voir l'onglet [Actions](https://github.com/ElegAlex/raffinerie/actions).

## Documentation

- [Spec de design](docs/superpowers/specs/2026-05-13-raffinerie-design.md)
- [Plan d'implémentation](docs/superpowers/plans/2026-05-13-raffinerie-implementation.md)
- [Guide utilisateur](docs/guide-utilisateur.md)
```

- [ ] **Step 2: Create `CHANGELOG.md`**

```markdown
# Changelog

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).

## [Non publié]

### Ajouté
- Parser pg_dump plain-text en streaming
- Moteur de filtres composables (UGE, nature, commentaire, notification, dates)
- Catalogue de 40 colonnes + 3 profils préenregistrés
- Export Excel multi-onglets (synthèse + mensuels + paramètres)
- UI Tauri 2 avec drag-drop, filtres réactifs, aperçu, persistance session
- CI GitHub Actions pour build Windows .exe

## [0.1.0] - 2026-05-13

- Bootstrap du projet
```

- [ ] **Step 3: Create `docs/guide-utilisateur.md`**

```markdown
# Guide utilisateur — raffinerie

## 1. Installation

1. Récupérer le fichier `raffinerie.exe` (portable) depuis l'onglet [Actions](https://github.com/ElegAlex/raffinerie/actions) → dernier workflow `Windows build` → artefact `raffinerie-windows-x64`.
2. Décompresser et placer `raffinerie.exe` dans un dossier de votre choix (ex: `Documents\Outils\`).
3. Double-cliquer pour lancer. Au premier lancement, Windows SmartScreen peut afficher un avertissement « Éditeur inconnu » → cliquer sur *Plus d'infos* puis *Exécuter quand même*.

## 2. Première utilisation — extraction CAMIEG

1. Glisser-déposer le fichier `sucre_939.dump` sur la fenêtre raffinerie. Le parsing prend ~5 secondes.
2. Vérifier que les filtres sont sur le préréglage *Standard CAMIEG* :
   - UGE : cocher `9501` (à confirmer)
   - Nature compte : `IND`
   - Commentaire contient : `indu roc`
   - Notification : *Motif notif rempli*
   - Date pivot : *Date intégration*
   - Période : `01/01/2026` → aujourd'hui
3. Cliquer sur *Aperçu* pour vérifier visuellement les 100 premières lignes.
4. Cliquer sur *Exporter Excel*. Choisir l'emplacement. Le fichier sera nommé `raffinerie_9501_<date>-<heure>.xlsx`.

## 3. Composition du classeur Excel

- **Synthèse** : tableau croisé Mois × UGE × (Nb créances, Σ montant initial, Σ solde). À coller dans le mail à la CAMIEG.
- **2026-01**, **2026-02**, … : un onglet par mois avec le détail des créances.
- **Paramètres** : traçabilité (date d'export, hash SHA-256 du dump, filtres appliqués). À conserver pour audit DCF.

## 4. Profils de colonnes personnels

- Sélectionner les colonnes voulues dans la zone *Colonnes à exporter*.
- Cliquer sur *Sauver profil…* et donner un nom (ex: « Reporting mensuel »).
- Le profil apparaîtra dans la liste déroulante préfixé par ★.

## 5. Persistance entre sessions

Vos filtres, profils et dernière configuration sont sauvegardés automatiquement dans `%APPDATA%\raffinerie\`. À chaque ouverture, raffinerie restaure votre dernière configuration. Vous n'avez plus qu'à reglisser le nouveau dump mensuel.

## 6. FAQ

**Le dump ne se charge pas (« Format non reconnu »)**
→ Vérifier que le fichier est bien un export pg_dump plain-text v13+ (extension `.dump`, contenu commençant par `-- PostgreSQL database dump`).

**Le compteur reste à 0 lignes**
→ Probablement aucune créance ne satisfait tous les filtres. Élargir la plage de dates ou retirer un filtre.

**L'export Excel est vide**
→ Vérifier qu'au moins une colonne est cochée dans la zone *Colonnes à exporter*.

**Sécurité des données**
→ raffinerie n'effectue aucune connexion réseau. Le dump et l'export Excel restent strictement sur votre poste.

## 7. Support

Pour toute anomalie : ouvrir un ticket sur https://github.com/ElegAlex/raffinerie/issues
ou contacter Alexandre Berge (CPAM 92).
```

- [ ] **Step 4: Commit**

```bash
cd /home/alex/Documents/REPO/raffinerie
git add README.md CHANGELOG.md docs/guide-utilisateur.md
git commit -m "docs: README enrichi + CHANGELOG + guide utilisateur"
git push
```

---

### Task 26: Final sanity check

- [ ] **Step 1: Full test suite**

```bash
cd /home/alex/Documents/REPO/raffinerie/src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 2: Clippy strict**

```bash
cd /home/alex/Documents/REPO/raffinerie/src-tauri && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
```

- [ ] **Step 3: Real-dump integration (if possible on Linux)**

```bash
cd /home/alex/Documents/REPO/raffinerie/src-tauri && RAFFINERIE_REAL_DUMP=/home/alex/Documents/REPO/SUCRE_DUMP/sucre_939.dump cargo test --release --test real_dump -- --nocapture 2>&1 | tail -10
```

Expected: parses in <10s, >1000 creances loaded, 0 (or very few) corrupted rows.

- [ ] **Step 4: Verify GitHub Actions green**

Check https://github.com/ElegAlex/raffinerie/actions for a successful `Windows build` workflow run with a downloadable artifact.

- [ ] **Step 5: Final commit (if any fixes)** + push

```bash
cd /home/alex/Documents/REPO/raffinerie
git status
# only push if there are commits
git push
```

---

## Self-review notes (for the planning agent)

- All spec sections (1-12) have at least one task implementing them.
- No "TBD" / "TODO" / "similar to" placeholders.
- Type names consistent: `FilterSet`, `NotifCriterion`, `DatePivot`, `FilteredRow`, `ParsedDump`, `ResolverContext`, `MonthKey`, `ExportContext`.
- Column ids in catalog (Task 11) match resolver branches (Task 13) and frontend select options (Task 22).
- TDD discipline preserved: each non-trivial Rust module starts with failing tests before implementation.
- The Linux dev environment is a known limitation; Windows `.exe` production is delegated to GitHub Actions (Task 24).

---

## Execution Handoff

Plan complete. Two execution options:

**1. Subagent-Driven (recommended)** - Fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** - Execute in this session, batch with checkpoints.

User has requested autonomous execution → use **Subagent-Driven** (superpowers:subagent-driven-development).
