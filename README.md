# raffinerie

Extracteur autonome de créances **SUCRE** depuis un dump `pg_dump`, avec filtres multi-UGE et export Excel multi-onglets.

> Le dump brut entre, le produit raffiné (XLSX exploitable) sort.

## Quoi

Application desktop Windows (Tauri + Rust) qui lit un dump PostgreSQL plain-text de la base SUCRE, applique des filtres métier paramétrables, et produit un classeur Excel structuré (synthèse + onglets mensuels + paramètres).

Conçue à l'origine pour répondre à la demande CAMIEG d'extraction mensuelle des indus ROC notifiés (mai 2026), mais utilisable pour toute extraction filtrée sur le dump SUCRE d'une caisse.

## Pourquoi

Les exports natifs SUCRE ne permettent pas de filtrer simultanément sur :
- le commentaire ordonnateur,
- le statut de notification,
- une plage de dates,
- avec regroupement mensuel.

Et le circuit officiel de demande d'évolution est à l'horizon Q1 2027. Cet outil comble le besoin immédiat.

## Statut

🚧 En cours de développement. Voir [`docs/superpowers/specs/`](docs/superpowers/specs/) pour le design validé.

## Stack

- **Backend** : Rust stable
- **Framework** : Tauri 2 (WebView2)
- **Frontend** : HTML + CSS + Alpine.js
- **Export** : `rust_xlsxwriter`
- **Build Windows .exe** : GitHub Actions runner `windows-latest`

## Build

### Pré-requis

- Rust stable (`rustup`)
- Node n'est PAS requis (pas de build step front).
- Sur Windows : WebView2 Runtime (préinstallé sur Win10 22H2+ / Win11).

### Dév local (Linux/macOS)

```sh
cd src-tauri
cargo test          # parser, filter, exporter
cargo run           # lance l'app en dev (Tauri auto-rechargement)
```

### Build .exe Windows

Push vers `main` → GitHub Actions produit le `.exe` portable dans les artefacts du workflow `windows-build`.

Ou en local sur Windows :

```sh
cargo tauri build
# artefact : src-tauri/target/release/raffinerie.exe
```

## Sécurité

- Aucune connexion réseau émise par l'application.
- Le dump et l'export Excel restent strictement locaux.
- Pas de logs persistants contenant des données nominatives.

## Licence

Usage interne CPAM / Assurance Maladie. Non publié sous licence open source pour l'instant.
