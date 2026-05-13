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

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotifCriterion {
    #[default]
    Aucun,
    MotifNotifNonVide,
    DateArNotifNonVide,
    EtapeWfDans {
        ids: Vec<i32>,
    },
    StatutCompteDans {
        values: Vec<String>,
    },
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
