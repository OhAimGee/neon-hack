//! The tactical intrusion of a site, turned from data into a playable [`Mission`].

use std::collections::BTreeSet;

use crate::content::Content;
use crate::content::ids::{ItemId, SiteId};
use crate::content::schema::{KitDef, RunDef, SiteDef};
use crate::run::{Mission, MissionBuilder, MissionError, NodeId, Pace, Program, Supply};

/// The tier a site belongs to: the one that unlocks it (1 when it is not gated by a tier).
#[must_use]
pub fn site_tier(site: &SiteDef) -> u8 {
    site.unlock.tier.unwrap_or(1)
}

impl RunDef {
    /// The node written with this number.
    #[must_use]
    pub fn node_id(number: u8) -> Option<NodeId> {
        NodeId::from_number(number)
    }

    /// The files of the site that the loot of a node extracts.
    #[must_use]
    pub fn files_at(&self, node: NodeId) -> &[crate::content::ids::FileId] {
        self.node.get(node.index()).map_or(&[], |n| &n.files)
    }

    /// Builds the mission, with the programs given and the pace of a difficulty.
    ///
    /// # Errors
    ///
    /// The [`MissionError`] that makes the data unplayable.
    pub fn mission(
        &self,
        kit: &KitDef,
        programs: &[Program],
        pace: Pace,
    ) -> Result<Mission, MissionError> {
        self.mission_boosted(kit, programs, &[], pace)
    }

    /// Builds the mission like [`RunDef::mission`], with one more charge for each program of
    /// `boosts` that the deck has with a limited supply (never above the most a supply may
    /// hold).
    ///
    /// # Errors
    ///
    /// The [`MissionError`] that makes the data unplayable.
    pub fn mission_boosted(
        &self,
        kit: &KitDef,
        programs: &[Program],
        boosts: &[Program],
        pace: Pace,
    ) -> Result<Mission, MissionError> {
        let node = |number: u8| Self::node_id(number).ok_or(MissionError::UnknownNode(number));
        let mut builder = MissionBuilder::new();
        for def in &self.node {
            builder.node(def.ice, def.loot);
        }
        for [a, b] in &self.link {
            builder.link(node(*a)?, node(*b)?);
        }
        builder.start(node(self.start)?);
        for patrol in &self.patrol {
            let route = patrol
                .route
                .iter()
                .map(|number| node(*number))
                .collect::<Result<Vec<_>, _>>()?;
            builder.patrol(&route, patrol.anchor.map(node).transpose()?);
        }
        builder.programs(programs);
        for boost in boosts {
            if programs.contains(boost)
                && let Supply::Limited(count) = boost.spec().supply
            {
                let raised = count.saturating_add(1).min(Supply::MAX_LIMITED);
                builder.program_with(*boost, Supply::Limited(raised));
            }
        }
        builder
            .cycles_per_turn(kit.cycles.saturating_add(pace.bonus_cycles))
            .gain_quarters(pace.gain_quarters)
            .ambient(self.ambient)
            .intel_known(self.intel_known);
        builder.build()
    }
}

/// What a player holds, for the deck of an intrusion.
#[derive(Debug, Clone, Copy)]
pub enum Stock<'a> {
    /// Everything the stall sells: the whole deck of the tier (the tools and the proofs use it).
    Everything,
    /// The items the player owns.
    Owned(&'a BTreeSet<ItemId>),
}

impl Content {
    /// Whether some item grants the program: then it is sold at the stall.
    #[must_use]
    pub fn is_sold(&self, program: Program) -> bool {
        self.items.iter().any(|item| item.program == Some(program))
    }

    /// The programs of a tier's deck that a player holding `stock` has.
    #[must_use]
    pub fn deck(&self, kit: &KitDef, stock: Stock<'_>) -> Vec<Program> {
        kit.programs
            .iter()
            .copied()
            .filter(|program| match stock {
                Stock::Everything => true,
                Stock::Owned(owned) => {
                    !self.is_sold(*program)
                        || self
                            .items
                            .iter()
                            .any(|item| item.program == Some(*program) && owned.contains(&item.id))
                }
            })
            .collect()
    }

    /// The programs whose supply the upgrades a player holds raise: every upgrade of the stall
    /// for [`Stock::Everything`], the owned ones otherwise.
    #[must_use]
    pub fn boosts(&self, stock: Stock<'_>) -> Vec<Program> {
        self.items
            .iter()
            .filter(|item| match stock {
                Stock::Everything => true,
                Stock::Owned(owned) => owned.contains(&item.id),
            })
            .filter_map(|item| item.boost)
            .collect()
    }

    /// The mission of a site at a pace with the whole deck of its tier: `None` for a site that
    /// has no intrusion.
    ///
    /// The data is checked when it is loaded, so a mission that does not build is a bug of the
    /// validation, which its tests rule out for the shipped content at every pace.
    #[must_use]
    pub fn mission(&self, site: &SiteId, pace: Pace) -> Option<Mission> {
        self.mission_holding(site, pace, Stock::Everything)
    }

    /// The mission of a site at a pace with the deck of a player who holds `stock`.
    #[must_use]
    pub fn mission_holding(&self, site: &SiteId, pace: Pace, stock: Stock<'_>) -> Option<Mission> {
        let def = self.site(site)?;
        let run = self.run(site)?;
        let kit = self.kit(site_tier(def))?;
        run.mission_boosted(kit, &self.deck(kit, stock), &self.boosts(stock), pace)
            .ok()
    }
}
