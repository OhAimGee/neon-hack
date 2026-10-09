//! Conversations: `accept`, `talk`, and the menu of an open conversation.
//!
//! A conversation is one `Talked` fact and a menu. The menu lists the dialogue topics of the
//! contact (open, already discussed, or not yet), then what the contact can be paid for, then
//! the decisions that are theirs to put to the player. Paying and deciding are menu entries
//! because the content language asks for a `pay` objective or a `choice` objective but names no
//! command for them: the person you pay, or answer, is the one you are talking to.

use crate::command::ArgRef;
use crate::content::Fact;
use crate::content::engine::{offered_choices, topic_available};
use crate::content::eval::applicable;
use crate::content::ids::{ContactId, DecisionId, QuestId};
use crate::content::money::Credits;
use crate::content::schema::{DecisionDef, Goal, QuestDef, QuestStatus, TopicDef};
use crate::event::Event;
use crate::prompt::{Choice, ChoiceOption, Input, Resolution};
use crate::text::Text;

use super::CampaignGame;
use super::dispatch::{arg_id, invalid};
use super::keys;
use super::state::Flow;

/// Where a dialogue topic stands for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopicState {
    Open,
    Discussed,
    NotYet,
}

/// One entry of a conversation menu.
enum Entry {
    Topic(&'static TopicDef, TopicState),
    Pay(&'static QuestDef, Credits),
    Decide(&'static DecisionDef),
}

impl CampaignGame {
    pub(super) fn cmd_accept(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(quest) = arg_id::<QuestId>(args, 0) else {
            return;
        };
        let (at, mut facts) = self.tick();
        facts.push(Fact::QuestAccepted { quest });
        self.apply(facts, at, events);
    }

    pub(super) fn cmd_talk(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(contact) = arg_id::<ContactId>(args, 0) else {
            return;
        };
        let (at, mut facts) = self.tick();
        facts.push(Fact::Talked {
            contact: contact.clone(),
            at,
        });
        self.apply(facts, at, events);
        if self.entries(&contact).is_empty() {
            events.push(Event::system(
                Text::new("campaign.talk.silent").with_text("name", keys::contact_name(&contact)),
            ));
        } else {
            self.state.flows.push(Flow::Talk { contact });
        }
    }

    /// What the conversation with a contact can be about, in menu order.
    fn entries(&self, contact: &ContactId) -> Vec<Entry> {
        let c = self.content;
        let missions = &self.state.missions;
        let mut entries: Vec<Entry> = c
            .topics
            .iter()
            .filter(|topic| topic.contact == *contact)
            .map(|topic| {
                let chosen = missions
                    .topics
                    .contains(&(topic.contact.clone(), topic.id.clone()));
                let state = if chosen {
                    TopicState::Discussed
                } else if topic_available(c, missions, topic) {
                    TopicState::Open
                } else {
                    TopicState::NotYet
                };
                Entry::Topic(topic, state)
            })
            .collect();
        for quest in c.quests.iter().filter(|q| q.giver == *contact) {
            if let Some(price) = self.pending_payment(quest) {
                entries.push(Entry::Pay(quest, price));
            }
        }
        for decision in &c.decisions {
            if c.quest(&decision.quest)
                .is_some_and(|q| q.giver == *contact)
                && self.decision_pending(decision)
            {
                entries.push(Entry::Decide(decision));
            }
        }
        entries
    }

    /// What the quest still asks to be paid, if it is active and has a `pay` objective that
    /// applies and is not done (directly, or as an alternative of an `any_of`).
    fn pending_payment(&self, quest: &QuestDef) -> Option<Credits> {
        let (c, missions) = (self.content, &self.state.missions);
        if missions.status(&quest.id) != QuestStatus::Active {
            return None;
        }
        let run = missions.quests.get(&quest.id)?;
        for (i, objective) in quest.objective.iter().enumerate() {
            let i = u8::try_from(i).ok()?;
            if run.done.contains(&(i, 0)) || !applicable(c, missions, objective) {
                continue;
            }
            let price = |goal: &Goal| match goal {
                Goal::Pay { amount } => Some(c.resolve(*amount, quest.tier).0),
                _ => None,
            };
            if let Some(found) = price(&objective.goal) {
                return Some(found);
            }
            for (j, alternative) in objective.children().iter().enumerate() {
                let j = u8::try_from(j + 1).ok()?;
                if !run.done.contains(&(i, j))
                    && applicable(c, missions, alternative)
                    && let Some(found) = price(&alternative.goal)
                {
                    return Some(found);
                }
            }
        }
        None
    }

    /// A decision is put to the player when its quest is active, nothing was decided yet and
    /// at least one option is offered.
    fn decision_pending(&self, decision: &DecisionDef) -> bool {
        let missions = &self.state.missions;
        missions.status(&decision.quest) == QuestStatus::Active
            && !missions.decisions.contains_key(&decision.id)
            && !offered_choices(self.content, missions, &decision.id).is_empty()
    }

    /// The menu of a conversation, derived from the state.
    pub(super) fn talk_choice(&self, contact: &ContactId) -> Choice {
        let credits = self.state.credits(self.content);
        let options = self
            .entries(contact)
            .iter()
            .map(|entry| match entry {
                Entry::Topic(topic, state) => ChoiceOption {
                    id: format!("topic:{}", topic.id),
                    label: match state {
                        TopicState::NotYet => Text::new("campaign.talk.hidden"),
                        _ => keys::topic_question(contact, &topic.id),
                    },
                    available: match state {
                        TopicState::Open => Ok(()),
                        TopicState::Discussed => Err(Text::new("campaign.reason.discussed")),
                        TopicState::NotYet => Err(Text::new("campaign.reason.topic_not_yet")),
                    },
                },
                Entry::Pay(quest, price) => ChoiceOption {
                    id: format!("pay:{}", quest.id),
                    label: Text::new("campaign.talk.pay")
                        .with_int("amount", i64::from(price.get()))
                        .with_text("title", keys::quest_title_short(&quest.id)),
                    available: if credits >= *price {
                        Ok(())
                    } else {
                        Err(Text::new("campaign.reason.credits")
                            .with_int("missing", i64::from(price.saturating_sub(credits).get())))
                    },
                },
                Entry::Decide(decision) => ChoiceOption {
                    id: format!("decide:{}", decision.id),
                    label: Text::new("campaign.talk.decide")
                        .with_text("prompt", keys::decision_prompt(&decision.id)),
                    available: Ok(()),
                },
            })
            .collect();
        Choice {
            title: Text::new("campaign.talk.title").with_text("name", keys::contact_name(contact)),
            options,
            cancel: Some(Text::new("campaign.talk.cancel")),
        }
    }

    pub(super) fn talk_menu(
        &mut self,
        contact: &ContactId,
        input: &Input,
        events: &mut Vec<Event>,
    ) {
        let choice = self.talk_choice(contact);
        match choice.resolve(input) {
            Resolution::Cancel => {
                self.state.flows.pop();
                events.push(Event::system(Text::new("campaign.talk.end")));
            }
            Resolution::Invalid => events.push(Event::error(Text::new("ui.invalid_choice"))),
            Resolution::Pick(index) => {
                let refused = choice
                    .options
                    .get(index)
                    .and_then(|option| option.available.clone().err());
                if let Some(reason) = refused {
                    events.push(Event::error(reason));
                    return;
                }
                let entries = self.entries(contact);
                match entries.get(index) {
                    Some(Entry::Topic(topic, _)) => self.choose_topic(contact, topic, events),
                    Some(Entry::Pay(quest, price)) => self.pay(quest, *price, events),
                    Some(Entry::Decide(decision)) => {
                        // About to decide: the player may come back to the moment before.
                        self.checkpoint = true;
                        self.state.flows.push(Flow::Decision {
                            decision: decision.id.clone(),
                        });
                        return;
                    }
                    None => invalid(events),
                }
                self.close_when_done(contact, events);
            }
        }
    }

    /// A conversation with nothing left to choose ends by itself.
    fn close_when_done(&mut self, contact: &ContactId, events: &mut Vec<Event>) {
        let left = self
            .talk_choice(contact)
            .options
            .iter()
            .any(|o| o.available.is_ok());
        if !left
            && self.state.flows.last()
                == Some(&Flow::Talk {
                    contact: contact.clone(),
                })
        {
            self.state.flows.pop();
            events.push(Event::system(Text::new("campaign.talk.done")));
        }
    }

    fn choose_topic(&mut self, contact: &ContactId, topic: &TopicDef, events: &mut Vec<Event>) {
        events.push(Event::narration(
            Text::new("campaign.talk.ask")
                .with_str("handle", self.state.handle.clone())
                .with_text("question", keys::topic_question(contact, &topic.id)),
        ));
        events.push(Event::say(
            contact.clone(),
            keys::topic_answer(contact, &topic.id),
        ));
        let (at, mut facts) = self.tick();
        facts.push(Fact::TopicChosen {
            contact: contact.clone(),
            topic: topic.id.clone(),
        });
        self.apply(facts, at, events);
    }

    fn pay(&mut self, quest: &QuestDef, price: Credits, events: &mut Vec<Event>) {
        events.push(Event::system(
            Text::new("campaign.talk.paid")
                .with_int("amount", i64::from(price.get()))
                .with_text("title", keys::quest_title_short(&quest.id)),
        ));
        let (at, mut facts) = self.tick();
        facts.push(Fact::Paid { amount: price, at });
        self.apply(facts, at, events);
    }

    // ------------------------------------------------------------------- decisions

    /// The options of a decision, derived from the state.
    pub(super) fn decision_choice(&self, decision: &DecisionId) -> Choice {
        let options = offered_choices(self.content, &self.state.missions, decision)
            .into_iter()
            .map(|choice| ChoiceOption {
                id: choice.id.to_string(),
                label: keys::decision_choice(decision, &choice.id),
                available: Ok(()),
            })
            .collect();
        Choice {
            title: keys::decision_prompt(decision),
            options,
            cancel: Some(Text::new("campaign.decision.cancel")),
        }
    }

    pub(super) fn decision_menu(
        &mut self,
        decision: &DecisionId,
        input: &Input,
        events: &mut Vec<Event>,
    ) {
        let choice = self.decision_choice(decision);
        match choice.resolve(input) {
            Resolution::Cancel => {
                self.state.flows.pop();
            }
            Resolution::Invalid => events.push(Event::error(Text::new("ui.invalid_choice"))),
            Resolution::Pick(index) => {
                let picked = choice
                    .options
                    .get(index)
                    .and_then(|option| option.id.parse().ok());
                self.state.flows.pop();
                let Some(picked) = picked else {
                    invalid(events);
                    return;
                };
                let (at, mut facts) = self.tick();
                facts.push(Fact::DecisionMade {
                    decision: decision.clone(),
                    choice: picked,
                });
                self.apply(facts, at, events);
                // Back in the conversation, which may have nothing left to say.
                if let Some(Flow::Talk { contact }) = self.state.flows.last().cloned() {
                    self.close_when_done(&contact, events);
                }
            }
        }
    }
}
