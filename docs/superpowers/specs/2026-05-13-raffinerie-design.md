# raffinerie — Design

**Date** : 2026-05-13
**Auteur** : Alexandre Berge (CPAM 92)
**Statut** : Design validé, en attente de plan d'implémentation

---

## 1. Contexte & motivation

### 1.1 Origine de la demande

Demande directe du DG et du DCF de la **CAMIEG**, relayée par Aurélie COLLOMB (Directrice du Cabinet de la Direction Générale, CPAM 92) :

> Extraction mensuelle depuis janvier 2026 des **créances notifiées** portant le commentaire ordonnateur **« indu ROC »**, sur l'UGE concernée par le dispositif ROC.

Cible métier : refacturation annuelle des prestations conventionnées à la CAMIEG par la CPAM 92.

### 1.2 Refus du circuit officiel SUCRE

La demande a été refusée par le Copil SUCRE national (mail Mathieu MESSE, PO SK-PHP, 12/05/2026) :

- Les évolutions fonctionnelles 2026 de SUCRE sont livrées (v4.1.1, MEP 02/05/2026).
- Besoin jugé « spécifique CAMIEG ».
- Renvoi au comité utilisateurs annuel du 19/05/2026 (évolutions développées Q4 2026 / Q1 2027).

Le Cabinet DG CPAM 92 a tranché : la demande CAMIEG doit être satisfaite sans attendre, **hors circuit officiel**, à partir du dump PostgreSQL de la base SUCRE locale (caisse 939 = CPAM 92).

### 1.3 Ressource d'entrée

Fichier `sucre_939.dump`, **dump pg_dump plain-text** (PostgreSQL v13.5), ~340 Mo, 50 tables, encodage UTF-8.

### 1.4 Livrable

Application desktop Windows autonome (`.exe`) qui :

- lit le dump SUCRE injecté par l'utilisateur,
- applique des filtres métiers paramétrables (UGE, commentaire, notification, dates),
- produit un classeur Excel structuré pour transmission à la CAMIEG.

Pas de récurrence automatisée : l'outil tourne à la demande, chaque mois, sur un dump frais fourni par l'utilisateur.

---

## 2. Périmètre fonctionnel

### 2.1 Inclus

- Parsing direct du dump SQL (zéro restauration Postgres).
- Filtre **multi-UGE** (scalable au-delà du cas CAMIEG initial).
- Filtre par **commentaire ordonnateur** (texte libre, insensible casse/accents).
- Filtre par **statut de notification** avec **5 critères au choix de l'utilisateur** (voir §4.3).
- Filtre par **plage de dates** sur une **date pivot configurable** (6 options).
- **Sélecteur de colonnes** à cases à cocher (catalogue de ~40 colonnes issues de 5 tables).
- **3 profils de colonnes** préenregistrés + sauvegarde de profils personnels.
- Export **Excel multi-onglets** : synthèse + 1 onglet par mois + onglet paramètres.
- Persistance de la dernière session (filtres + profil) pour relance rapide.

### 2.2 Exclu (YAGNI)

- Pas de connexion à une base PostgreSQL en direct.
- Pas de mécanisme auto-update.
- Pas de multi-langue (français uniquement).
- Pas de tests UI automatisés.
- Pas d'export CSV/PDF (Excel uniquement).
- Pas de gestion d'utilisateurs/permissions interne à l'app.

---

## 3. Architecture générale

### 3.1 Stack technique

- **Framework** : Tauri (Rust + WebView2).
- **Backend** : Rust stable, structure modulaire (parser / schema / filter / exporter).
- **Frontend** : HTML/CSS + Alpine.js (10 ko, pas de build step front).
- **Excel** : crate `rust_xlsxwriter`.
- **Dates** : crate `chrono`.

### 3.2 Justification du choix Tauri

- WebView2 v145 confirmé présent sur le poste utilisateur cible (vérification `reg query` effectuée 13/05/2026).
- Postes CPAM 92 modernes (Win10 22H2 / Win11) ont WebView2 préinstallé via la politique DSI Edge Chromium.
- UI riche (cases à cocher, drag-drop, modale aperçu) plus ergonomique en HTML qu'en egui/iced.
- `.exe` portable ~8-12 Mo, démarrage <1s.
- Parser Rust → 340 Mo lus en 3-5 s.

**Fallback documenté** : si un poste cible se révèle sans WebView2, basculer sur Rust + egui (single binary 15-25 Mo, zéro dépendance runtime). Pas d'implémentation prévue pour l'instant.

### 3.3 Diagramme

```
┌─────────────────────────────────────────────────────┐
│  Frontend (WebView2 / HTML+CSS+Alpine.js)           │
│  • Drop zone .dump                                  │
│  • Panneau filtres (UGE, nature, commentaire,       │
│    notif, date pivot, plage dates)                  │
│  • Catalogue colonnes (cases à cocher par groupe)   │
│  • Aperçu 100 lignes + Export Excel                 │
└──────────────────────┬──────────────────────────────┘
                       │ Tauri IPC (invoke commands)
┌──────────────────────▼──────────────────────────────┐
│  Backend Rust (binaire unique)                      │
│  ├─ parser/      streaming pg_dump COPY blocks      │
│  ├─ schema/      structs des 5 tables utiles        │
│  ├─ filter/      FilterSet composable               │
│  ├─ aggregator/  regroupement mensuel               │
│  └─ exporter/    génération XLSX multi-onglets      │
└─────────────────────────────────────────────────────┘
```

### 3.4 Tables chargées en mémoire

Seules **5 tables** sur les 50 du dump sont chargées :

| Table | Rôle | Volume estimé |
|---|---|---|
| `creance` | Table principale (1 ligne = 1 créance) | ~50 000 lignes |
| `creance_regroupee` | Porte « Indu ROC » + dates notif + motif notif | ~30 000 lignes |
| `uge` | Libellés UGE (résolution code → nom) | ~50 lignes |
| `etapeworkflow` | Libellés étapes workflow | ~200 lignes |
| `adresse_debiteur` | Optionnel, si colonnes adresse cochées | ~20 000 lignes |

Estimation mémoire totale en RAM : 80-120 Mo. Les 45 autres tables sont skippées au parsing.

### 3.5 Indexation post-parsing

Trois `HashMap` construits après chargement pour jointures O(1) :

- `HashMap<i64, &CreanceRegroupee>` indexé par `creance_regroupee.id`.
- `HashMap<String, &Uge>` indexé par `uge.num_uge`.
- `HashMap<i32, &EtapeWorkflow>` indexé par `etapeworkflow.id`.

---

## 4. Parser SQL

### 4.1 Format ciblé

pg_dump plain-text, version 13+. Les données sont dans des blocs :

```
COPY public.<table> (col1, col2, ...) FROM stdin;
<ligne1 tab-séparée>
<ligne2 tab-séparée>
...
\.
```

### 4.2 Logique de parsing

```
BufReader ligne par ligne sur le .dump (capacité 64 ko)
  ├─ Hors bloc COPY :
  │   • Détecter "COPY public.<table_utile> (cols...)"
  │   • Si table NON utile → skip jusqu'au prochain "\." (zéro alloc data)
  │   • Si table utile → parser l'ordre des colonnes (l'ordre dans le COPY
  │     peut différer du CREATE TABLE)
  ├─ Dans bloc COPY d'une table utile :
  │   • Split sur '\t'
  │   • Décodage escapes pg_dump : \N → NULL, \\ → \, \t → tab, \n → newline,
  │     \r → CR
  │   • Construction d'un struct typé (parsing date YYYY-MM-DD, numeric → f64,
  │     bool t/f)
  │   • Push dans Vec<T>
  └─ Sortie : 5 Vec<T>
```

### 4.3 Gestion d'erreurs

| Cas | Comportement |
|---|---|
| Fichier non lisible | Toast erreur « Impossible d'ouvrir le fichier » + détail |
| Format inconnu (pas pg_dump) | « Format non reconnu : attendu pg_dump plain-text v13+ » |
| Table requise absente | « Table `creance` introuvable dans le dump — dump incomplet ? » |
| Colonne attendue absente d'un COPY | Log warning + colonne valorisée à NULL pour toutes les lignes |
| Ligne corrompue (nb colonnes incorrect) | Skip + incrément compteur, affiché en fin de parsing |
| Date invalide | NULL + log warning |

### 4.4 Performance attendue

- Parsing complet : 3-5 s sur 340 Mo sur poste CPAM standard.
- Mémoire pic : ~150 Mo (parsing + structs).
- UI réactive pendant parsing (thread séparé, event `parsing-progress` émis tous les 5 %).

### 4.5 Tests parser

- Fixtures de dump minimaux (~50 ko) couvrant : chaque escape, valeurs NULL, dates valides/invalides, ligne corrompue, table inutile à skipper, ordre de colonnes inversé.
- Tests unitaires : `escape_decode()`, `parse_copy_header()`, `parse_row()`.
- Test d'intégration : `parse_full_fixture()` qui charge les 5 tables et vérifie les comptes.

---

## 5. Moteur de filtres

### 5.1 Modèle

```rust
struct FilterSet {
    uges: Option<Vec<String>>,
    nature_compte: Option<Vec<String>>,
    commentaire_contient: Option<String>,
    commentaire_case_insensitive: bool,  // défaut true
    notif_criterion: NotifCriterion,
    date_pivot: DatePivot,
    date_min: Option<NaiveDate>,
    date_max: Option<NaiveDate>,
}

enum NotifCriterion {
    Aucun,
    MotifNotifNonVide,
    DateArNotifNonVide,
    EtapeWfDans(Vec<i32>),
    StatutCompteDans(Vec<String>),
}

enum DatePivot {
    DateDetect,             // creance.date_detect
    DateIntegration,        // creance.date_integration  (DÉFAUT)
    DateDerOpe,             // creance.date_der_ope
    DateMandatement,        // creance.date_mandatement
    DateArNotifDebiteur,    // creance_regroupee.date_ar_notif_debiteur
    DateDetectionRegroupee, // creance_regroupee.date_detection
}
```

### 5.2 Évaluation

- Itération unique sur `creance` (~50 000 lignes).
- Court-circuit dès qu'un filtre échoue.
- Filtres sur `creance_regroupee` (commentaire, motif notif, date AR notif, étape WF) : résolution via HashMap `creanceregroupeeid` → O(1).
- Recalcul du compteur de lignes (« 1 247 lignes ») à chaque modif de filtre, debounce 200 ms.
- Performance cible : <200 ms pour le filtrage complet.

### 5.3 Cas par défaut (premier lancement, parcours CAMIEG)

À l'ouverture sans session précédente, les filtres sont pré-renseignés :

- UGE : aucune sélectionnée (l'utilisateur choisit selon le besoin).
- Nature compte : `IND`.
- Commentaire contient : `indu roc` (insensible casse/accents).
- Notification : `Motif notif rempli`.
- Date pivot : `date_integration`.
- Période : 2026-01-01 → aujourd'hui.

---

## 6. Catalogue de colonnes & profils

### 6.1 Catalogue (groupé par section dans l'UI)

**Créance** : `numero_creance`, `nature_compte`, `statut_compte`, `montant_initial`, `solde`, `part_mutuel`, `type_prest`, `arc_det`, `nature_der_ope`, `flux`, `activite`, `num_compte`, `commentaire_creance` *(de la table `creance`)*, `num_technicien`.

**Dates** : `date_detect`, `date_integration`, `date_der_ope`, `date_mandatement`, `date_prescription`.

**UGE** : `num_uge_gestion`, `num_uge_detect`, `libelle_uge` *(résolu via table `uge`)*.

**Débiteur** : `numero_debiteur`, `cat_debiteur`, `nom_assure`, `prenom_assure`, `matricule_assure`, et optionnellement (via `adresse_debiteur`) : `adresse`, `code_postal`, `commune`.

**Regroupée** : `numero_reference`, `commentaire_creance_regroupee` *(c'est ce champ qui porte « Indu ROC »)*, `motif_notif`, `date_detection`, `date_ar_notif_debiteur`, `date_ar_mdm_debiteur`, `etapewf` + `libelle_etape` *(résolu)*, `is_douteux`, `numero_og3s`.

**Désambiguïsation** : `creance.commentaire_creance` ≠ `creance_regroupee.commentaire_creance`. Affichés dans l'UI comme `Commentaire (créance)` et `Commentaire regroupée`. Le second est le filtre métier ROC.

### 6.2 Profils préenregistrés

**Standard CAMIEG** (par défaut, à valider avec Corinne BOURUMEAU) :

- N° créance, N° débiteur, Nom assuré, Prénom assuré, Matricule assuré, Montant initial, Solde, Date détection, Date AR notif, Commentaire regroupée, N° UGE gestion.

**Complet** : toutes les colonnes du catalogue.

**Minimal** : N° créance, Montant initial, *date pivot* (suit le choix utilisateur), Commentaire regroupée.

### 6.3 Profils personnels

- Bouton « Sauver comme profil personnel ».
- Stockage : `%APPDATA%\raffinerie\profiles.json` (Windows). Format JSON simple : `{ "nom_profil": ["col1", "col2", ...] }`.
- Profils personnels listés dans le dropdown après les 3 préenregistrés.

---

## 7. Export Excel

### 7.1 Structure du classeur

1. **Onglet `Synthèse`** (en tête) — tableau croisé prêt à coller dans le mail CAMIEG :
   - Lignes : mois (`2026-01`, `2026-02`, …).
   - Colonnes : par UGE sélectionnée, 3 indicateurs (`Nb créances`, `Σ Montant initial`, `Σ Solde`).
   - Ligne `Total` en bas.

2. **N onglets mensuels** : un par mois traversé par la plage de dates, nommés `2026-01`, `2026-02`, …
   - Contiennent les lignes filtrées dont la `date_pivot` tombe dans le mois.
   - Colonnes = celles cochées par l'utilisateur.

3. **Onglet `Paramètres`** (traçabilité audit DCF) :
   - Date d'export, version de l'outil, nom du fichier dump source, taille + hash SHA-256 du dump (preuve d'intégrité).
   - Récap exhaustif des filtres appliqués (UGE, dates, critère notif, commentaire, etc.).
   - Liste des colonnes exportées.
   - Compteurs : lignes lues, lignes filtrées, lignes corrompues skippées au parsing.

### 7.2 Formats Excel

- Montants : `# ##0,00 €` (locale fr_FR).
- Dates : `JJ/MM/AAAA`.
- Booléens : `Oui` / `Non`.
- Entête figé (`freeze_panes` row 1) sur tous les onglets de données.
- Filtres auto (`autofilter`) sur toutes les colonnes des onglets mensuels.
- Largeurs auto-ajustées au contenu (max 60 caractères).
- Police entête : gras + fond gris clair.

### 7.3 Nommage fichier

`raffinerie_<UGE1-UGE2>_<aaaammjj-HHMM>.xlsx`

Exemple : `raffinerie_9501-9531_20260513-1042.xlsx`.

Si aucune UGE filtrée : `raffinerie_toutesUGE_<aaaammjj-HHMM>.xlsx`.

Dialog « Enregistrer sous » natif Windows, défaut = `Documents/`.

### 7.4 Performance

- Génération XLSX en background (thread Rust).
- Toast de progression (« Écriture onglet 2026-03… »).
- Toast final avec boutons « Ouvrir le fichier » et « Ouvrir le dossier ».
- Cible : <3 s pour 10 000 lignes × 15 colonnes × 5 onglets.

---

## 8. UI Tauri — parcours utilisateur

### 8.1 Structure visuelle

Une fenêtre, 3 zones empilées (pas de wizard) :

1. **Zone fichier** : drop zone + chemin + état de parsing.
2. **Zone filtres** : UGE / nature / commentaire / notif / date pivot / période.
3. **Zone colonnes** : sélecteur par groupe replié + dropdown profils.
4. **Zone action** : compteur estimé + boutons Aperçu / Exporter.

(Maquette ASCII détaillée dans le compte-rendu de brainstorming associé, repris ici en commentaire dans le code front.)

### 8.2 Comportements clés

- Drag-drop fichier actif tant qu'aucun dump n'est chargé.
- Parsing en thread séparé, progress bar via events Tauri.
- Filtres réactifs : recalcul du compteur (« 1 247 lignes ») avec debounce 200 ms.
- Aperçu : modale 100 lignes pour validation avant export.
- Export : dialog natif puis génération background.
- Mode sombre suit le thème Windows.

### 8.3 Persistance session

- Fichier `%APPDATA%\raffinerie\last-session.json`.
- Sauvegardés : filtres, profil colonnes actif, date pivot.
- **Non** sauvegardés : chemin du dump (changeant chaque mois), résultats.
- Rechargés au démarrage suivant.

### 8.4 Internationalisation

Français uniquement. Aucun système i18n.

---

## 9. Packaging & livraison

### 9.1 Artefacts de build

`cargo tauri build --target x86_64-pc-windows-msvc` produit :

1. **`Raffinerie.exe` portable** (~8-12 Mo) — *livrable principal*, copier-coller, double-clic.
2. **`Raffinerie_1.0.0_x64-setup.exe`** — installateur NSIS avec raccourci Démarrer + désinstallation.
3. **`Raffinerie_1.0.0_x64.msi`** — pour déploiement DSI via SCCM si demandé.

### 9.2 Signature Authenticode

- Idéalement signer le `.exe` pour éviter le SmartScreen Windows.
- À voir avec DSI CPAM 92 : disponibilité d'un certif organisationnel.
- Si pas signé : popup SmartScreen au 1er lancement, contournable par 1 clic « Plus d'infos > Exécuter quand même ». À documenter dans la doc utilisateur.

### 9.3 Sécurité données

- Aucune connexion réseau émise par l'application — à vérifier en runtime et à documenter (RSSI).
- Pas de logs persistants contenant des données nominatives. Logs techniques uniquement (parsing, erreurs) dans `%APPDATA%\raffinerie\logs\`, rotation 7 fichiers max.
- Le dump et l'export Excel restent strictement sur le poste utilisateur.

### 9.4 Documentation livrée

- `README.md` (racine du repo).
- `Guide utilisateur.pdf` (5-6 pages, captures d'écran, parcours « extraction CAMIEG mensuelle »).
- `CHANGELOG.md` versionné.

### 9.5 Structure du repo

```
raffinerie/
├── src-tauri/              ← backend Rust
│   ├── src/
│   │   ├── main.rs
│   │   ├── parser/         ← parsing pg_dump COPY
│   │   ├── schema/         ← structs des 5 tables
│   │   ├── filter/         ← FilterSet
│   │   ├── aggregator/     ← regroupement mensuel
│   │   └── exporter/       ← XLSX
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                    ← frontend
│   ├── index.html
│   ├── app.js              (Alpine.js)
│   └── styles.css
├── tests/
│   └── fixtures/           ← mini-dumps de test
├── docs/
│   ├── superpowers/specs/  ← ce document
│   └── guide-utilisateur.md
├── README.md
└── CHANGELOG.md
```

---

## 10. Tests

- **Parser** : unitaires sur escapes / parsing date / parsing numeric / lignes corrompues / colonnes absentes. Intégration sur fixtures de dump minimaux.
- **Filter** : unitaires sur chaque variant `NotifCriterion`, chaque `DatePivot`, combinaisons AND.
- **Exporter** : intégration — fixtures → XLSX généré → relecture via `calamine` et vérification des cellules clés.
- **UI** : pas de tests automatisés. Validation manuelle sur parcours CAMIEG.

---

## 11. Roadmap (post-implémentation initiale)

Hors scope du livrable v1.0 mais à anticiper :

- v1.1 : support d'autres caisses (dumps `sucre_XXX.dump`) — devrait fonctionner sans modif si le schéma est identique.
- v1.2 : option d'export CSV pour les utilisateurs qui le demandent.
- v1.3 : profils colonnes partagés (export/import JSON) pour standardisation inter-services.

---

## 12. Points ouverts résiduels

| Point | À traiter |
|---|---|
| UGE cible CAMIEG : **9501** (Aurélie) ou **9531** (Corinne) ? | **TRANCHÉ par analyse du dump (2026-05-13)** : UGE 9531 = "Pôle Camieg" dans le référentiel SUCRE. UGE 9501 n'existe pas. 523 indus ROC trouvés sur UGE 9531 (via `num_uge_detect`, le `num_uge_gestion` valant "0"). Le filtre raffinerie matche désormais sur gestion OR detect pour gérer ce cas. |
| Profil « Standard CAMIEG » : colonnes exactes attendues ? | Soumettre la liste proposée à Corinne pour validation. |
| Signature Authenticode | Demande à formuler à la DSI CPAM 92. |
| Test WebView2 sur poste Corinne | À faire avant livraison v1.0. |
