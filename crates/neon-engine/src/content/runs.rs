//! The tactical intrusion of a site, turned from data into a playable [`Mission`].

use crate::content::Content;
use crate::content::ids::SiteId;
use crate::content::schema::{KitDef, RunDef, SiteDef};
use crate::run::{Mission, MissionBuilder, MissionError, NodeId, Pace};

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

    /// Builds the mission, with the deck of a tier and the pace of a difficulty.
    ///
    /// # Errors
    ///
    /// The [`MissionError`] that makes the data unplayable.
    pub fn mission(&self, kit: &KitDef, pace: Pace) -> Result<Mission, MissionError> {
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
        builder
            .programs(&kit.programs)
            .cycles_per_turn(kit.cycles.saturating_add(pace.bonus_cycles))
            .gain_quarters(pace.gain_quarters)
            .ambient(self.ambient)
            .intel_known(self.intel_known);
        builder.build()
    }
}

impl Content {
    /// The mission of a site at a pace: `None` for a site that has no intrusion.
    ///
    /// The data is checked when it is loaded, so a mission that does not build is a bug of the
    /// validation, which its tests rule out for the shipped content at every pace.
    #[must_use]
    pub fn mission(&self, site: &SiteId, pace: Pace) -> Option<Mission> {
        let def = self.site(site)?;
        let run = self.run(site)?;
        let kit = self.kit(site_tier(def))?;
        run.mission(kit, pace).ok()
    }
}
