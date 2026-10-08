use proptest::prelude::*;

use super::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Wallet {
    credits: Credits,
    reputation: Reputation,
}

#[test]
fn credits_saturate_at_both_ends() {
    let big = Credits::new(CREDITS_CAP - 1);
    assert_eq!(big.saturating_add(Credits::new(5)), Credits::CAP);
    assert_eq!(
        Credits::new(3).saturating_sub(Credits::new(5)),
        Credits::ZERO
    );
    assert_eq!(Credits::new(u32::MAX), Credits::CAP);
    assert_eq!(Credits::CAP.saturating_add(Credits::CAP), Credits::CAP);
}

#[test]
fn a_percentage_rounds_down_and_cannot_overflow() {
    assert_eq!(Credits::new(150).percent(50).get(), 75);
    assert_eq!(Credits::new(101).percent(50).get(), 50);
    assert_eq!(Credits::CAP.percent(u32::MAX), Credits::CAP);
    assert_eq!(Credits::new(80).percent(0), Credits::ZERO);
}

#[test]
fn reputation_saturates_at_both_ends() {
    let top = Reputation::new(REPUTATION_CAP);
    assert_eq!(top.saturating_add(Reputation::new(1)), top);
    let bottom = top.negated();
    assert_eq!(bottom.saturating_add(Reputation::new(-1)), bottom);
    assert_eq!(Reputation::new(i32::MIN), bottom);
    assert_eq!(Reputation::new(10).negated().get(), -10);
}

#[test]
fn the_text_form_is_a_plain_integer_and_out_of_range_values_are_refused() {
    let wallet = Wallet {
        credits: Credits::new(120),
        reputation: Reputation::new(-5),
    };
    let text = toml::to_string(&wallet).unwrap();
    assert_eq!(text, "credits = 120\nreputation = -5\n");
    assert_eq!(toml::from_str::<Wallet>(&text).unwrap(), wallet);

    for bad in [
        "credits = 1000000001\nreputation = 0",
        "credits = -1\nreputation = 0",
        "credits = 1\nreputation = 1000001",
        "credits = 1\nreputation = -1000001",
        "credits = 1\nreputation = 99999999999",
    ] {
        assert!(toml::from_str::<Wallet>(bad).is_err(), "{bad}");
    }
    let edge = "credits = 1000000000\nreputation = -1000000";
    assert!(toml::from_str::<Wallet>(edge).is_ok());
}

proptest! {
    #[test]
    fn arithmetic_with_extreme_amounts_stays_in_range(a in any::<u32>(), b in any::<u32>(), p in any::<u32>()) {
        let (x, y) = (Credits::new(a), Credits::new(b));
        prop_assert!(x.saturating_add(y).get() <= CREDITS_CAP);
        prop_assert!(x.saturating_sub(y) <= x);
        prop_assert!(x.percent(p).get() <= CREDITS_CAP);
    }

    #[test]
    fn reputation_arithmetic_with_extreme_values_stays_in_range(a in any::<i32>(), b in any::<i32>()) {
        let sum = Reputation::new(a).saturating_add(Reputation::new(b)).get();
        prop_assert!((-REPUTATION_CAP..=REPUTATION_CAP).contains(&sum));
        let flipped = Reputation::new(a).negated().get();
        prop_assert!((-REPUTATION_CAP..=REPUTATION_CAP).contains(&flipped));
    }

    #[test]
    fn what_is_written_is_read_back(a in 0..=CREDITS_CAP, r in -REPUTATION_CAP..=REPUTATION_CAP) {
        let wallet = Wallet { credits: Credits::new(a), reputation: Reputation::new(r) };
        let back: Wallet = toml::from_str(&toml::to_string(&wallet).unwrap()).unwrap();
        prop_assert_eq!(back, wallet);
    }
}
