//! Shared vocabulary for the bill-vs-spending classification.
//!
//! Keeps the Swedish labels and the `<select>` value encoding in one place so
//! the inline pickers and the guided review screen cannot drift apart.

use api::models::{CostKind, Matching, Periodicity};

/// Options offered in a cost-kind picker, in display order.
pub const COST_KINDS: [CostKind; 4] = [
    CostKind::Recurring(Periodicity::Monthly),
    CostKind::Recurring(Periodicity::Quarterly),
    CostKind::Recurring(Periodicity::Annual),
    CostKind::Variable,
];

pub fn cost_kind_slug(kind: CostKind) -> &'static str {
    match kind {
        // No picker offers `OneOff`; it only reaches here from legacy data and
        // is shown as the nearest real option.
        CostKind::Recurring(Periodicity::Monthly | Periodicity::OneOff) => "monthly",
        CostKind::Recurring(Periodicity::Quarterly) => "quarterly",
        CostKind::Recurring(Periodicity::Annual) => "annual",
        CostKind::Variable => "variable",
    }
}

pub fn cost_kind_from_slug(slug: &str) -> CostKind {
    match slug {
        "quarterly" => CostKind::Recurring(Periodicity::Quarterly),
        "annual" => CostKind::Recurring(Periodicity::Annual),
        "variable" => CostKind::Variable,
        _ => CostKind::Recurring(Periodicity::Monthly),
    }
}

pub fn cost_kind_label(kind: CostKind) -> &'static str {
    match kind {
        CostKind::Recurring(Periodicity::Monthly | Periodicity::OneOff) => "Månadsräkning",
        CostKind::Recurring(Periodicity::Quarterly) => "Kvartalsräkning",
        CostKind::Recurring(Periodicity::Annual) => "Årsräkning",
        CostKind::Variable => "Rörlig utgift",
    }
}

/// One line explaining what the choice changes, for the review screen.
pub fn cost_kind_hint(kind: CostKind) -> &'static str {
    match kind {
        CostKind::Recurring(Periodicity::Quarterly | Periodicity::Annual) => {
            "Periodiseras per månad och byggs upp som buffert till räkningen kommer."
        }
        CostKind::Recurring(_) => "Budgeteras med samma belopp varje månad.",
        CostKind::Variable => "Budgeteras per månad efter hur mycket du faktiskt handlar.",
    }
}

pub fn cost_kind_class(kind: CostKind) -> &'static str {
    match kind {
        CostKind::Recurring(Periodicity::Monthly | Periodicity::OneOff) => "cbi-periodicity-badge",
        CostKind::Recurring(Periodicity::Quarterly) => "cbi-periodicity-badge quarterly",
        CostKind::Recurring(Periodicity::Annual) => "cbi-periodicity-badge annual",
        CostKind::Variable => "cbi-periodicity-badge oneoff",
    }
}

/// Sort order for a cost-kind column: bills first, shortest cycle first.
pub fn cost_kind_sort_key(kind: CostKind) -> u8 {
    match kind {
        CostKind::Recurring(Periodicity::Monthly | Periodicity::OneOff) => 0,
        CostKind::Recurring(Periodicity::Quarterly) => 1,
        CostKind::Recurring(Periodicity::Annual) => 2,
        CostKind::Variable => 3,
    }
}

pub fn matching_label(matching: Matching) -> &'static str {
    match matching {
        Matching::Automatic => "Taggas automatiskt",
        Matching::Suggest => "Föreslås för godkännande",
    }
}

pub fn matching_hint(matching: Matching) -> &'static str {
    match matching {
        Matching::Automatic => {
            "Regeln appliceras direkt vid import — bra när betalningstexten alltid ser likadan ut."
        }
        Matching::Suggest => {
            "Träffar visas som förslag du får godkänna — bra när samma butik kan vara olika saker."
        }
    }
}

pub fn matching_slug(matching: Matching) -> &'static str {
    match matching {
        Matching::Automatic => "automatic",
        Matching::Suggest => "suggest",
    }
}

pub fn matching_from_slug(slug: &str) -> Matching {
    match slug {
        "automatic" => Matching::Automatic,
        _ => Matching::Suggest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_kind_slugs_round_trip() {
        for kind in COST_KINDS {
            assert_eq!(cost_kind_from_slug(cost_kind_slug(kind)), kind);
        }
    }

    #[test]
    fn matching_slugs_round_trip() {
        for m in [Matching::Automatic, Matching::Suggest] {
            assert_eq!(matching_from_slug(matching_slug(m)), m);
        }
    }

    #[test]
    fn legacy_one_off_maps_onto_a_real_option() {
        // `OneOff` is never offered, but legacy data must still render and must
        // round-trip to something the picker can display.
        let slug = cost_kind_slug(CostKind::Recurring(Periodicity::OneOff));
        assert!(COST_KINDS.contains(&cost_kind_from_slug(slug)));
    }

    #[test]
    fn every_option_is_distinctly_labelled() {
        let labels: Vec<&str> = COST_KINDS.iter().copied().map(cost_kind_label).collect();
        let unique: std::collections::HashSet<&&str> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }
}
