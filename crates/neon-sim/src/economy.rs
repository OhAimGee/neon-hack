//! The economy of the shipped content, read from the data: what the stall sells and for how
//! much, what the services cost, and what the quests pay at each tier.
//!
//! Nothing is played here: the campaign reports say what the bots earned, this says what the
//! content offers and asks. The thresholds of the economy (T7) read both.

use std::collections::BTreeSet;

use neon_engine::content::Content;
use neon_engine::content::schema::{Goal, Objective, QuestKind};
use serde::{Deserialize, Serialize};

/// An item of the stall.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRow {
    /// The item.
    pub id: String,
    /// The tier from which it is sold.
    pub tier: u8,
    /// Its price.
    pub price: u32,
    /// The quests that ask the player to own it.
    pub required_by: Vec<String>,
}

/// A service sold by `laylow`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRow {
    /// The service.
    pub id: String,
    /// Its price.
    pub price: u32,
    /// The Notoriety points it removes.
    pub cooling: u8,
}

/// What the quests of a tier pay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierRow {
    /// The tier.
    pub tier: u8,
    /// `R(P)`, the price of its flagship item.
    pub flagship_price: u32,
    /// The credits the main quests of the tier pay.
    pub main_credits: u32,
    /// The credits every quest of the tier pays (the side ones included, the exclusive
    /// branches too: an upper bound).
    pub all_credits: u32,
}

/// The economy of the content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyReport {
    /// The version of the format.
    pub schema_version: u32,
    /// The credits the player starts with.
    pub start_credits: u32,
    /// What the whole stall costs: the items that stay.
    pub catalogue_total: u32,
    /// The items.
    pub items: Vec<ItemRow>,
    /// The services.
    pub services: Vec<ServiceRow>,
    /// The tiers.
    pub tiers: Vec<TierRow>,
}

/// The items a goal asks to own, the alternatives of an `any_of` included.
fn owned_by(objective: &Objective, out: &mut BTreeSet<String>) {
    match &objective.goal {
        Goal::Buy { item } => {
            out.insert(item.to_string());
        }
        Goal::AnyOf(of) => of.iter().for_each(|child| owned_by(child, out)),
        _ => {}
    }
}

/// Reads the economy from the content.
#[must_use]
pub fn measure(content: &Content) -> EconomyReport {
    let items: Vec<ItemRow> = content
        .items
        .iter()
        .map(|item| {
            let required_by = content
                .quests
                .iter()
                .filter(|quest| {
                    let mut asked = BTreeSet::new();
                    quest
                        .objective
                        .iter()
                        .for_each(|objective| owned_by(objective, &mut asked));
                    asked.contains(item.id.as_str())
                })
                .map(|quest| quest.id.to_string())
                .collect();
            ItemRow {
                id: item.id.to_string(),
                tier: item.unlock.tier.unwrap_or(1),
                price: item.price.get(),
                required_by,
            }
        })
        .collect();
    let services = content
        .services
        .iter()
        .map(|service| ServiceRow {
            id: service.id.to_string(),
            price: service.price.get(),
            cooling: service.cooling,
        })
        .collect();
    let tiers = content
        .tiers
        .iter()
        .map(|tier| {
            let paid = |main_only: bool| -> u32 {
                content
                    .quests
                    .iter()
                    .filter(|quest| quest.tier == tier.id)
                    .filter(|quest| !main_only || quest.kind == QuestKind::Main)
                    .filter_map(|quest| quest.reward.as_ref()?.credits)
                    .map(|size| content.resolve(size, tier.id).0.get())
                    .sum()
            };
            TierRow {
                tier: tier.id,
                flagship_price: tier.price.get(),
                main_credits: paid(true),
                all_credits: paid(false),
            }
        })
        .collect();
    EconomyReport {
        schema_version: crate::report::SCHEMA_VERSION,
        start_credits: content.rules.start_credits.get(),
        catalogue_total: items.iter().map(|item| item.price).sum(),
        items,
        services,
        tiers,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use super::*;

    #[test]
    fn the_stall_the_services_and_the_tiers_are_read_from_the_content() {
        let content = Content::embedded().unwrap();
        let report = measure(&content);
        assert_eq!(report.items.len(), content.items.len());
        assert_eq!(
            report.catalogue_total,
            report.items.iter().map(|i| i.price).sum::<u32>()
        );
        assert!(report.start_credits > 0);
        assert_eq!(report.services.len(), content.services.len());
        assert_eq!(report.tiers.len(), content.tiers.len());
        for tier in &report.tiers {
            assert!(tier.all_credits >= tier.main_credits, "{tier:?}");
        }
        // The first tier pays out something, and the flagship prices are those of the data.
        assert!(report.tiers.iter().any(|t| t.main_credits > 0));
        assert_eq!(report.tiers[0].flagship_price, 80);
    }

    #[test]
    fn the_items_a_quest_asks_for_are_told_apart_from_the_others() {
        let report = measure(&Content::embedded().unwrap());
        let asked = |id: &str| {
            report
                .items
                .iter()
                .find(|item| item.id == id)
                .is_some_and(|item| !item.required_by.is_empty())
        };
        // The second item of the stall is a step of the story; the first is a choice.
        assert!(asked("stealth_module"));
        assert!(!asked("ghost_protocol"));
    }
}
