# Changelog

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).

## [Non publié]

### Ajouté
- Parser pg_dump plain-text en streaming (~2,3 s sur 340 Mo, 497k créances, 0 lignes corrompues sur le dump de référence)
- 5 tables chargées en mémoire : `creance`, `creance_regroupee`, `uge`, `etapeworkflow`, `adresse_debiteur`
- Moteur de filtres composables (UGE, nature, commentaire, notification, dates) avec jointure O(1) via HashMap
- Recherche commentaire insensible casse + accents (normalisation NFD)
- Catalogue de 40 colonnes + 3 profils préenregistrés (Standard CAMIEG, Complet, Minimal)
- Profils personnels persistés dans `%APPDATA%\raffinerie\profiles.json`
- Persistance de la dernière session (filtres + colonnes) entre lancements
- Export Excel multi-onglets : synthèse (croisé mois × UGE) + N onglets mensuels + onglet paramètres (traçabilité audit avec SHA-256 du dump source)
- UI Tauri 2 avec drag-drop fichier, filtres réactifs, aperçu 100 lignes, sélecteur de colonnes groupé par catégorie, thème sombre
- IPC Tauri : 14 commandes (parse, list_*, count, preview, export, profiles, session)
- CI GitHub Actions : build Windows .exe portable + installeur NSIS sur runner `windows-latest`

## [0.1.0] - 2026-05-13

- Bootstrap du projet (spec, plan, scaffolding Tauri 2)
