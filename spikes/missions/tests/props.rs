//! Properties of the evaluator and of the loader.

mod common;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use neon_spike_missions::engine::{Output, refresh};
use neon_spike_missions::optimist::{all_plans, play};
use neon_spike_missions::schema::SiteMark;
use neon_spike_missions::{Content, Fact, Sources, State, new_game};
use proptest::prelude::*;

fn content() -> &'static Content {
    static C: OnceLock<Content> = OnceLock::new();
    C.get_or_init(common::shipped)
}

/// Facts built from the ids of the shipped content, plus a few unknown ids.
fn fact() -> BoxedStrategy<Fact> {
    let c = content();
    let sites: Vec<String> = c
        .sites
        .iter()
        .map(|s| s.id.to_string())
        .chain(["nowhere".to_owned()])
        .collect();
    let files: Vec<(String, String)> = c
        .sites
        .iter()
        .flat_map(|s| s.file.iter().map(|f| (s.id.to_string(), f.id.to_string())))
        .collect();
    let readables: Vec<String> = c.readables.iter().map(|r| r.id.to_string()).collect();
    let items: Vec<String> = c.items.iter().map(|i| i.id.to_string()).collect();
    let contacts: Vec<String> = c.contacts.iter().map(|x| x.id.to_string()).collect();
    let commands: Vec<String> = c.commands.iter().map(|x| x.id.to_string()).collect();
    let choices: Vec<(String, String)> = c
        .decisions
        .iter()
        .flat_map(|d| {
            d.choice
                .iter()
                .map(|x| (d.id.to_string(), x.id.to_string()))
        })
        .collect();
    let topics: Vec<(String, String)> = c
        .topics
        .iter()
        .map(|t| (t.contact.to_string(), t.id.to_string()))
        .collect();
    let quests: Vec<String> = c.quests.iter().map(|x| x.id.to_string()).collect();
    let at = 1u32..40;
    prop_oneof![
        3 => (proptest::sample::select(sites.clone()), at.clone()).prop_map(|(s, at)| Fact::SiteCompromised { site: s.as_str().into(), at }),
        3 => proptest::sample::select(files).prop_map(|(s, f)| Fact::FileExtracted { site: s.as_str().into(), file: f.as_str().into() }),
        1 => (proptest::sample::select(sites), prop_oneof![Just(SiteMark::Backdoor), Just(SiteMark::Virus), Just(SiteMark::Analyzed)])
            .prop_map(|(s, mark)| Fact::SiteMarked { site: s.as_str().into(), mark }),
        2 => proptest::sample::select(readables.clone()).prop_map(|r| Fact::Read { id: r.as_str().into() }),
        2 => proptest::sample::select(readables).prop_map(|r| Fact::Decrypted { id: r.as_str().into() }),
        2 => proptest::sample::select(items).prop_map(|i| Fact::ItemBought { item: i.as_str().into() }),
        3 => (proptest::sample::select(contacts.clone()), at.clone()).prop_map(|(x, at)| Fact::Talked { contact: x.as_str().into(), at }),
        1 => proptest::sample::select(commands).prop_map(|x| Fact::CommandUsed { command: x.as_str().into() }),
        1 => (proptest::sample::select(contacts), 1u8..4).prop_map(|(x, level)| Fact::LinkChanged { contact: x.as_str().into(), level }),
        1 => (prop_oneof![Just(400u32), Just(800), Just(1600)], at.clone()).prop_map(|(amount, at)| Fact::Paid { amount, at }),
        2 => (0u8..101, at.clone()).prop_map(|(heat, at)| Fact::HeatChanged { heat, at }),
        2 => proptest::sample::select(choices).prop_map(|(d, ch)| Fact::DecisionMade { decision: d.as_str().into(), choice: ch.as_str().into() }),
        1 => proptest::sample::select(topics).prop_map(|(x, t)| Fact::TopicChosen { contact: x.as_str().into(), topic: t.as_str().into() }),
        2 => proptest::sample::select(quests.clone()).prop_map(|x| Fact::QuestAccepted { quest: x.as_str().into() }),
        1 => (proptest::sample::select(quests), at.clone()).prop_map(|(x, at)| Fact::QuestRestarted { quest: x.as_str().into(), at }),
        1 => at.prop_map(|at| Fact::Tick { at }),
    ]
    .boxed()
}

/// Legal histories: the rounds of the optimistic player for a few plans. Random facts alone
/// almost never complete a quest, so the properties start from a prefix of a real play.
fn traces() -> &'static Vec<Vec<Vec<Fact>>> {
    static T: OnceLock<Vec<Vec<Vec<Fact>>>> = OnceLock::new();
    T.get_or_init(|| {
        let c = content();
        all_plans(c)
            .iter()
            .step_by(31)
            .map(|p| play(c, p).log)
            .collect()
    })
}

/// `(trace, cut)`: which play, and after how many rounds.
type Warm = (usize, usize);

fn warm() -> impl Strategy<Value = Warm> {
    (0..8usize, 0..80usize)
}

fn prefix(w: Warm) -> Vec<Vec<Fact>> {
    let t = traces();
    t.get(w.0 % t.len())
        .map(|rounds| rounds.iter().take(w.1).cloned().collect())
        .unwrap_or_default()
}

/// The next round of the play after the prefix.
fn next_round(w: Warm) -> Vec<Fact> {
    let t = traces();
    t.get(w.0 % t.len())
        .and_then(|rounds| rounds.get(w.1))
        .cloned()
        .unwrap_or_default()
}

/// A state reached by the prefix of a real play, then random batches, each followed by a refresh.
fn reached(w: Warm, history: &[Vec<Fact>]) -> (State, Vec<Output>) {
    let c = content();
    let (mut s, mut out) = new_game(c);
    for batch in prefix(w).iter().chain(history) {
        for f in batch {
            s.apply(c, f);
        }
        out.extend(refresh(c, &mut s));
    }
    (s, out)
}

fn history() -> impl Strategy<Value = Vec<Vec<Fact>>> {
    proptest::collection::vec(proptest::collection::vec(fact(), 0..8), 0..4)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// `refresh` reaches a fixed point: a second call does nothing.
    #[test]
    fn refresh_is_idempotent(w in warm(), h in history()) {
        let c = content();
        let (mut s, out) = reached(w, &h);
        prop_assert!(!out.contains(&Output::BudgetExceeded));
        let before = s.clone();
        prop_assert!(refresh(c, &mut s).is_empty());
        prop_assert_eq!(s, before);
    }

    /// Recording a fact twice is recording it once.
    #[test]
    fn apply_is_idempotent(w in warm(), h in history(), f in fact()) {
        let c = content();
        let (mut once, _) = reached(w, &h);
        let mut twice = once.clone();
        once.apply(c, &f);
        twice.apply(c, &f);
        twice.apply(c, &f);
        prop_assert_eq!(once, twice);
    }

    /// The facts of one batch can arrive in any order.
    #[test]
    fn a_batch_of_facts_is_order_independent(
        w in warm(),
        h in history(),
        noise in proptest::collection::vec(fact(), 0..8),
        seed in any::<u64>(),
    ) {
        let c = content();
        let (base, _) = reached(w, &h);
        // The next real round of the play plus noise, in one order and in a shuffled one.
        let mut batch = next_round(w);
        batch.extend(noise);
        let mut shuffled = batch.clone();
        let mut x = seed | 1;
        for i in (1..shuffled.len()).rev() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            shuffled.swap(i, usize::try_from(x % (i as u64 + 1)).unwrap_or(0));
        }
        let (mut a, mut b) = (base.clone(), base);
        for f in &batch { a.apply(c, f); }
        for f in &shuffled { b.apply(c, f); }
        prop_assert_eq!(&a, &b, "the ledger depends on the order");
        let (oa, ob) = (refresh(c, &mut a), refresh(c, &mut b));
        prop_assert_eq!(oa, ob);
        prop_assert_eq!(a, b);
    }

    /// Whatever happens, a reward key is paid once and the credits add up.
    #[test]
    fn rewards_are_never_paid_twice(w in warm(), h in history(), again in history()) {
        let c = content();
        let (s, mut out) = reached(w, &h);
        // Replaying more (possibly the same) facts on top cannot pay anything twice.
        let mut s2 = s;
        for batch in &again {
            for f in batch { s2.apply(c, f); }
            out.extend(refresh(c, &mut s2));
        }
        let keys: Vec<&String> = out.iter().filter_map(|o| if let Output::Reward { key, .. } = o { Some(key) } else { None }).collect();
        let unique: BTreeSet<&&String> = keys.iter().collect();
        prop_assert_eq!(keys.len(), unique.len());
        let completed: Vec<_> = out.iter().filter(|o| matches!(o, Output::QuestCompleted(_))).collect();
        let distinct: BTreeSet<String> = completed.iter().map(|o| format!("{o:?}")).collect();
        prop_assert_eq!(completed.len(), distinct.len());
        let paid: i64 = out.iter().map(|o| if let Output::Reward { credits, .. } = o { *credits } else { 0 }).sum();
        prop_assert_eq!(s2.earned, i64::from(c.rules.start_credits) + paid);
    }

    /// Facts about things that do not exist change nothing.
    #[test]
    fn unknown_ids_are_ignored(w in warm(), h in history(), junk in "[a-z0-9_-]{1,12}") {
        let c = content();
        let (mut s, _) = reached(w, &h);
        let before = s.clone();
        let facts = [
            Fact::FileExtracted { site: junk.as_str().into(), file: "x".into() },
            Fact::ItemBought { item: junk.as_str().into() },
            Fact::Read { id: junk.as_str().into() },
            Fact::QuestAccepted { quest: junk.as_str().into() },
            Fact::DecisionMade { decision: junk.as_str().into(), choice: "x".into() },
            Fact::TopicChosen { contact: junk.as_str().into(), topic: "x".into() },
        ];
        for f in &facts { s.apply(c, f); }
        prop_assert_eq!(s, before);
    }
}

/// The generators are not vacuous: they complete quests, open them, make decisions, fail.
#[test]
fn generated_histories_reach_interesting_states() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = (warm(), history());
    let (mut completed, mut opened, mut decided, mut late) = (0, 0, 0, 0);
    for _ in 0..200 {
        let (w, h) = strategy
            .new_tree(&mut runner)
            .map(|t| t.current())
            .unwrap_or_default();
        let (s, out) = reached(w, &h);
        completed += usize::from(out.iter().any(|o| matches!(o, Output::QuestCompleted(_))));
        opened += usize::from(out.iter().any(|o| matches!(o, Output::QuestOpened(_))));
        decided += usize::from(!s.decisions.is_empty());
        late += usize::from(s.tier >= 5);
    }
    assert!(
        completed >= 100 && opened >= 100 && decided >= 20 && late >= 20,
        "{completed} {opened} {decided} {late}"
    );
}

fn mutate(text: &str, pos: usize, len: usize, junk: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = pos % (chars.len() + 1);
    let end = (start + len).min(chars.len());
    let mut out: String = chars.iter().take(start).collect();
    out.push_str(junk);
    out.extend(chars.iter().skip(end));
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Hostile or damaged data is an error, never a panic.
    #[test]
    fn hostile_toml_never_panics(
        file in 0usize..7,
        pos in 0usize..60_000,
        len in 0usize..300,
        junk in "\\PC{0,24}",
        noise in "\\PC{0,200}",
        replace_all in any::<bool>(),
    ) {
        let mut src = Sources::embedded();
        let slot: &mut String = match file {
            0 => &mut src.catalog,
            1 => &mut src.flags,
            2 => &mut src.rewards,
            3 => &mut src.quests,
            4 => &mut src.decisions,
            5 => &mut src.topics,
            _ => &mut src.texts,
        };
        *slot = if replace_all { noise } else { mutate(slot, pos, len, &junk) };
        let _ = Content::from_sources(&src);
    }

    /// Deeply nested or enormous values are refused without exhausting the stack.
    #[test]
    fn pathological_shapes_never_panic(depth in 1usize..200, width in 1usize..50) {
        let mut src = Sources::embedded();
        let cond = format!("{}{{ kind = \"quest\", id = \"m01\", is = \"completed\" }}{}", "{ kind = \"not\", of = ".repeat(depth), " }".repeat(depth));
        src.quests = src.quests.replacen("available_if = { kind = \"flag\", name = \"d2\", is = \"save\" }", &format!("available_if = {cond}"), 1);
        src.quests.push_str(&"[[quest]]\n".repeat(width));
        let _ = Content::from_sources(&src);
    }
}
