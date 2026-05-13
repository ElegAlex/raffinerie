# Guide utilisateur — raffinerie

## 1. Installation

1. Récupérer le fichier `raffinerie.exe` (portable) depuis l'onglet [Actions](https://github.com/ElegAlex/raffinerie/actions) du dépôt → dernier workflow `Windows build` → artefact `raffinerie-windows-x64`.
2. Décompresser et placer `raffinerie.exe` dans un dossier de votre choix (ex: `Documents\Outils\`).
3. Double-cliquer pour lancer. Au premier lancement, Windows SmartScreen peut afficher un avertissement « Éditeur inconnu » → cliquer sur *Plus d'infos* puis *Exécuter quand même*.

## 2. Première utilisation — extraction CAMIEG

1. Glisser-déposer le fichier `sucre_939.dump` sur la fenêtre raffinerie. Le parsing prend ~2-5 secondes.
2. Une fois le dump chargé, configurer les filtres :
   - **UGE** : cocher `9531` (Pôle Camieg — validé par l'analyse du dump : 523 indus ROC sur cette UGE)
   - **Nature compte** : `IND`
   - **Commentaire contient** : `indu roc` (insensible casse/accents activé)
   - **Notification** : *Motif notif rempli*
   - **Date pivot** : *Date intégration*
   - **Période** : `01/01/2026` → date du jour
3. Vérifier le compteur en bas (`Résultat estimé : … lignes`).
4. Cliquer sur **Aperçu** pour visualiser les 100 premières lignes.
5. Cliquer sur **Exporter Excel**. Choisir l'emplacement. Le fichier sera nommé `raffinerie_9531_<date>-<heure>.xlsx`.

## 3. Composition du classeur Excel

- **Synthèse** : tableau croisé Mois × UGE × (Nb créances, Σ montant initial, Σ solde). À coller dans le mail à la CAMIEG.
- **2026-01**, **2026-02**, … : un onglet par mois traversé par la période, avec les colonnes cochées.
- **Paramètres** : traçabilité de l'export (date, version raffinerie, chemin et SHA-256 du dump source, filtres appliqués, colonnes exportées, compteurs). À conserver pour audit DCF.

## 4. Profils de colonnes personnels

- Sélectionner les colonnes voulues dans la zone *Colonnes à exporter*.
- Cliquer sur **Sauver profil…** et donner un nom (ex: « Reporting mensuel »).
- Le profil apparaîtra dans la liste déroulante préfixé par ★.

## 5. Persistance entre sessions

Vos filtres, profils et dernière configuration sont sauvegardés automatiquement dans `%APPDATA%\raffinerie\`. À chaque ouverture, raffinerie restaure votre dernière configuration. Vous n'avez plus qu'à reglisser le nouveau dump mensuel.

## 6. FAQ

**Le dump ne se charge pas (« Format non reconnu »)**
→ Vérifier que le fichier est bien un export pg_dump plain-text v13+ (extension `.dump`, contenu commençant par `-- PostgreSQL database dump`).

**Le compteur reste à 0 lignes**
→ Probablement aucune créance ne satisfait tous les filtres. Élargir la plage de dates ou retirer un filtre.

**L'export Excel est désactivé (bouton grisé)**
→ Vérifier qu'au moins une colonne est cochée dans la zone *Colonnes à exporter* ET que le compteur est > 0.

**Le commentaire ne ramène rien alors que je sais qu'il existe**
→ Vérifier la case *insensible casse/accents*. SUCRE inscrit parfois « Indu ROC » avec une majuscule et un espace insécable — la recherche par défaut gère ces variations.

**Sécurité des données**
→ raffinerie n'effectue **aucune connexion réseau**. Le dump et l'export Excel restent strictement sur votre poste. À vérifier dans le pare-feu si nécessaire (aucune règle autorisée requise).

## 7. Support

- Pour toute anomalie : ouvrir un ticket sur https://github.com/ElegAlex/raffinerie/issues
- Contact métier : Alexandre Berge (CPAM 92)
