//! Buying: the shop and its confirmation, and the cover services of `laylow`.

use crate::command::ArgRef;
use crate::content::Fact;
use crate::content::ids::{ItemId, ServiceId};
use crate::event::Event;
use crate::prompt::Input;
use crate::text::Text;

use super::CampaignGame;
use super::dispatch::{arg_id, invalid};
use super::keys;
use super::state::Flow;

fn price_text(price: u32) -> Text {
    Text::new("campaign.price").with_int("price", i64::from(price))
}

impl CampaignGame {
    /// The vitrine: every item, with its price and whether it can be bought now, in words.
    pub(super) fn cmd_shop(&mut self, events: &mut Vec<Event>) {
        let rows = self
            .content
            .items
            .iter()
            .map(|item| {
                vec![
                    Text::raw(item.id.as_str()),
                    keys::item_name(&item.id),
                    price_text(item.price.get()),
                    self.state
                        .item_refusal(self.content, &item.id)
                        .unwrap_or_else(|| Text::new("campaign.shop.available")),
                ]
            })
            .collect();
        events.push(Event::Screen(crate::event::Table {
            title: Text::new("campaign.shop.title"),
            columns: [
                "campaign.col.id",
                "campaign.col.name",
                "campaign.col.price",
                "campaign.col.status",
            ]
            .iter()
            .map(|key| Text::new(key))
            .collect(),
            rows,
        }));
    }

    /// `buy`: every purchase is permanent, so the game asks, and keeps a checkpoint of the
    /// moment before.
    pub(super) fn cmd_buy(&mut self, args: &[ArgRef]) {
        let Some(item) = arg_id::<ItemId>(args, 0) else {
            return;
        };
        self.checkpoint = true;
        self.state.flows.push(Flow::ConfirmBuy { item });
    }

    /// The question of the purchase being confirmed.
    pub(super) fn buy_question(&self, item: &ItemId) -> Text {
        let price = self.content.item(item).map_or(0, |def| def.price.get());
        Text::new("campaign.buy.confirm")
            .with_text("name", keys::item_name(item))
            .with_int("price", i64::from(price))
    }

    pub(super) fn confirm_buy(&mut self, item: &ItemId, input: &Input, events: &mut Vec<Event>) {
        match input {
            Input::Confirm(true) => {
                self.state.flows.pop();
                if let Some(reason) = self.state.item_refusal(self.content, item) {
                    events.push(Event::error(reason));
                    return;
                }
                events.push(Event::reward(
                    Text::new("campaign.buy.done").with_text("name", keys::item_name(item)),
                ));
                let (at, mut facts) = self.tick();
                facts.push(Fact::ItemBought { item: item.clone() });
                self.apply(facts, at, events);
            }
            Input::Confirm(false) | Input::Cancel => {
                self.state.flows.pop();
                events.push(Event::system(Text::new("campaign.buy.cancelled")));
            }
            _ => invalid(events),
        }
    }

    /// `laylow`: the services that lower the notoriety, or buying one.
    pub(super) fn cmd_laylow(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        match arg_id::<ServiceId>(args, 0) {
            None => {
                let rows = self
                    .content
                    .services
                    .iter()
                    .map(|service| {
                        vec![
                            Text::raw(service.id.as_str()),
                            keys::service_name(&service.id),
                            price_text(service.price.get()),
                            Text::new("campaign.laylow.cooling")
                                .with_int("cooling", i64::from(service.cooling)),
                            self.state
                                .service_refusal(self.content, &service.id)
                                .unwrap_or_else(|| Text::new("campaign.shop.available")),
                        ]
                    })
                    .collect();
                events.push(Event::Screen(crate::event::Table {
                    title: Text::new("campaign.laylow.title"),
                    columns: [
                        "campaign.col.id",
                        "campaign.col.name",
                        "campaign.col.price",
                        "campaign.col.effect",
                        "campaign.col.status",
                    ]
                    .iter()
                    .map(|key| Text::new(key))
                    .collect(),
                    rows,
                }));
                self.used("laylow", events);
            }
            Some(id) => self.buy_service(&id, events),
        }
    }

    fn buy_service(&mut self, id: &ServiceId, events: &mut Vec<Event>) {
        let Some(service) = self.content.services.iter().find(|s| s.id == *id) else {
            return;
        };
        let (price, cooling) = (service.price, service.cooling);
        events.push(Event::system(
            Text::new("campaign.laylow.done")
                .with_text("name", keys::service_name(id))
                .with_int("cooling", i64::from(cooling)),
        ));
        self.state.spend(price);
        self.state.base_notoriety = self.state.base_notoriety.saturating_sub(cooling);
        let (at, mut facts) = self.tick();
        if let Ok(command) = "laylow".parse() {
            facts.push(Fact::CommandUsed { command });
        }
        self.apply(facts, at, events);
    }
}
