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
            ColumnDef {
                id: $id,
                label: $label,
                group: $group,
            }
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
        c!(
            "commentaire_creance_regroupee",
            "Commentaire regroupée",
            "Regroupée"
        ),
        c!("motif_notif", "Motif notification", "Regroupée"),
        c!(
            "date_detection_regroupee",
            "Date détection regroupée",
            "Regroupée"
        ),
        c!(
            "date_ar_notif_debiteur",
            "Date AR notification débiteur",
            "Regroupée"
        ),
        c!(
            "date_ar_mdm_debiteur",
            "Date AR mise en demeure",
            "Regroupée"
        ),
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
    vec![
        "numero_creance",
        "montant_initial",
        "date_integration",
        "commentaire_creance_regroupee",
    ]
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

    #[test]
    fn minimal_profile_ids_exist_in_catalog() {
        let ids: HashSet<&str> = catalog().iter().map(|c| c.id).collect();
        for id in profile_minimal() {
            assert!(ids.contains(id), "profile id {id} not in catalog");
        }
    }

    #[test]
    fn complet_profile_matches_catalog_size() {
        assert_eq!(profile_complet().len(), catalog().len());
    }
}
