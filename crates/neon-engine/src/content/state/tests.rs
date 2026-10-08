use super::*;

fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("shipped content is invalid:\n{e}"))
}

fn contact(id: &str) -> ContactId {
    id.parse().unwrap()
}

#[test]
fn a_new_state_starts_with_the_credits_and_contacts_of_the_content() {
    let c = shipped();
    let s = State::new(&c);
    assert_eq!(s.earned, c.rules.start_credits);
    assert_eq!(s.credits(&c), c.rules.start_credits);
    assert!(!s.overdrawn(&c));
    assert_eq!(s.contacts.len(), c.contacts.len());
    assert_eq!(s.contact(&contact("echo7")), ContactState::Available);
    assert_eq!(s.contact(&contact("nobody")), ContactState::Offline);
    assert_eq!(s.validate(&c), Ok(()));
}

#[test]
fn heat_readings_are_brought_back_to_the_gauge() {
    let c = shipped();
    let mut s = State::new(&c);
    s.apply(&c, &Fact::HeatChanged { heat: 255, at: 1 });
    assert_eq!(s.heat(), HEAT_MAX);
    assert_eq!(s.peak_since(0), HEAT_MAX);
}

#[test]
fn the_ledger_tells_when_more_was_spent_than_earned() {
    let c = shipped();
    let mut s = State::new(&c);
    let item = c
        .items
        .iter()
        .find(|i| i.price > s.earned)
        .expect("an item costs more than the starting credits");
    s.apply(
        &c,
        &Fact::ItemBought {
            item: item.id.clone(),
        },
    );
    assert!(s.overdrawn(&c));
    assert_eq!(s.credits(&c), Credits::ZERO, "never below zero");
}

#[test]
fn payments_and_purchases_add_up_without_overflow() {
    let c = shipped();
    let mut s = State::new(&c);
    for at in 1..=5 {
        s.apply(
            &c,
            &Fact::Paid {
                amount: Credits::CAP,
                at,
            },
        );
    }
    assert!(s.overdrawn(&c));
    assert_eq!(s.credits(&c), Credits::ZERO);
}

#[test]
fn trust_won_or_lost_saturates() {
    let c = shipped();
    let mut s = State::new(&c);
    let who = contact("echo7");
    for _ in 0..4 {
        s.add_trust(&who, i32::MAX);
    }
    assert_eq!(s.bonus_trust[&who], TRUST_CAP);
    for _ in 0..8 {
        s.add_trust(&who, i32::MIN);
    }
    assert_eq!(s.bonus_trust[&who], -TRUST_CAP);
    // The total still adds up without wrapping.
    let _ = s.trust(&c, &who);
}

#[test]
fn a_fact_with_an_unknown_id_changes_nothing_but_the_clock() {
    let c = shipped();
    let mut s = State::new(&c);
    let before = s.clone();
    let site = SiteId::new("nowhere").unwrap();
    assert!(!s.apply(
        &c,
        &Fact::SiteMarked {
            site,
            mark: SiteMark::Virus
        }
    ));
    assert_eq!(s, before);
    assert!(s.apply(&c, &Fact::Tick { at: 7 }));
    assert_eq!(s.clock, 7);
}

#[test]
fn facts_know_their_turn() {
    assert_eq!(Fact::Tick { at: 3 }.at(), Some(3));
    assert_eq!(
        Fact::CommandUsed {
            command: CommandId::new("scan").unwrap()
        }
        .at(),
        None
    );
}
