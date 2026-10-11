use proptest::prelude::*;

use super::*;

fn hack(site: &str) -> Staging {
    Staging::new(
        Effect::Hack {
            site: site.to_owned(),
            security: 3,
            taken: 2,
        },
        Some((10, 20)),
    )
}

fn laylow() -> Staging {
    Staging::new(Effect::Laylow { cooling: 10 }, Some((30, 20)))
}

/// Steps through a whole staging at the pace of the frames, collecting what each frame shows.
fn frames(mut staging: Staging) -> Vec<Staging> {
    let mut all = vec![staging.clone()];
    while !staging.is_over() {
        staging.tick(FRAME);
        all.push(staging.clone());
    }
    all
}

#[test]
fn a_staging_never_outlasts_the_ceiling() {
    for staging in [hack("localhost"), laylow()] {
        assert!(staging.duration() <= MAX_DURATION);
        let mut staging = staging;
        staging.tick(Duration::from_secs(3600));
        assert!(staging.is_over());
        assert_eq!(staging.elapsed, staging.duration(), "time stops at the end");
    }
}

#[test]
fn the_bar_starts_empty_ends_full_and_never_goes_back() {
    for staging in [hack("localhost"), laylow()] {
        let all = frames(staging);
        let first = all.first().unwrap();
        let last = all.last().unwrap();
        assert_eq!((first.percent(), first.is_done()), (0, false));
        assert_eq!((last.percent(), last.is_done()), (100, true));
        for pair in all.windows(2) {
            assert!(pair[0].progress() <= pair[1].progress(), "{pair:?}");
            assert!(pair[0].phase() <= pair[1].phase(), "{pair:?}");
        }
    }
}

#[test]
fn a_hundred_percent_is_held_for_a_moment_so_that_the_result_can_be_read() {
    let all = frames(hack("localhost"));
    let held = all
        .iter()
        .filter(|staging| staging.percent() == 100)
        .count();
    // About 300 ms at 50 ms a frame.
    assert!((4..=8).contains(&held), "{held} frames at 100 %");
}

#[test]
fn the_phases_follow_the_curve_and_the_last_one_is_reached() {
    let all = frames(hack("localhost"));
    let phases: Vec<Phase> = all.iter().map(Staging::phase).collect();
    assert_eq!(phases.first(), Some(&1));
    assert_eq!(phases.last(), Some(&hack("x").phases()));
    for wanted in 1..=hack("x").phases() {
        assert!(phases.contains(&wanted), "phase {wanted} is never shown");
    }
    assert_eq!(laylow().phases(), 3);
}

#[test]
fn the_intrusion_lingers_on_the_defences() {
    // The second step (bypassing the defences) takes the largest share of the time.
    let all = frames(hack("localhost"));
    let count = |phase: Phase| {
        all.iter()
            .filter(|staging| staging.phase() == phase)
            .count()
    };
    assert!(count(2) > count(1) && count(2) > count(3), "{}", count(2));
}

#[test]
fn the_notoriety_travels_from_before_to_after() {
    let all = frames(laylow());
    assert_eq!(all.first().unwrap().notoriety(), Some(30));
    assert_eq!(all.last().unwrap().notoriety(), Some(20));
    for pair in all.windows(2) {
        assert!(
            pair[0].notoriety() >= pair[1].notoriety(),
            "it only goes down"
        );
    }
    let none = Staging::new(Effect::Laylow { cooling: 1 }, None);
    assert_eq!(none.notoriety(), None);
}

#[test]
fn the_noise_is_the_same_for_the_same_moment_and_changes_between_moments() {
    let mut a = hack("localhost");
    let b = hack("localhost");
    assert_eq!(a.noise(0, 30), b.noise(0, 30));
    let before = a.noise(0, 30);
    a.tick(Duration::from_millis(500));
    assert_ne!(before, a.noise(0, 30));
    assert_ne!(
        hack("localhost").noise(0, 30),
        hack("corp-server-01").noise(0, 30)
    );
    assert_ne!(a.noise(0, 30), a.noise(1, 30), "rows differ");
}

#[test]
fn the_noise_keeps_its_face_between_two_steps_and_does_not_flicker() {
    // The figures change every NOISE_STEP_MS, not at every frame (MOT-1: nothing flickers).
    let all = frames(hack("localhost"));
    let mut faces: Vec<u64> = all.iter().map(Staging::noise_face).collect();
    faces.dedup();
    assert!(
        faces.len() < all.len() / 2,
        "{} faces in {} frames",
        faces.len(),
        all.len()
    );
    // Eight a second at most, far from anything that flashes.
    let seconds = hack("x").duration().as_secs_f64();
    let faces_f = f64::from(u32::try_from(faces.len()).unwrap());
    assert!(faces_f / seconds <= 10.0, "{} faces", faces.len());
    // Two frames inside one step show the same unsettled figures.
    let mut a = hack("localhost");
    a.tick(Duration::from_millis(500));
    let mut b = a.clone();
    b.tick(Duration::from_millis(50));
    assert_eq!(a.noise_face(), b.noise_face());
}

#[test]
fn the_settled_part_of_the_noise_grows_with_the_bar_and_never_changes() {
    let all = frames(hack("localhost"));
    let mut settled_before = String::new();
    for staging in &all {
        let noise = staging.noise(0, 30);
        assert_eq!(noise.locked.len() + noise.live.len(), 30);
        // What had settled stays as it was.
        assert!(noise.locked.starts_with(&settled_before), "{noise:?}");
        settled_before = noise.locked;
    }
    assert_eq!(all.last().unwrap().noise(0, 30).live, "");
    assert_eq!(all.first().unwrap().noise(0, 30).locked, "");
}

proptest! {
    /// Whatever the width, the row is made of hexadecimal digits and spaces: 7-bit, and
    /// exactly as wide as asked.
    #[test]
    fn the_noise_is_ascii_and_exactly_as_wide_as_asked(
        width in 0usize..200,
        row in 0usize..20,
        millis in 0u64..4000,
    ) {
        let mut staging = hack("nexus-mainframe");
        staging.tick(Duration::from_millis(millis));
        let noise = staging.noise(row, width);
        let all = format!("{}{}", noise.locked, noise.live);
        prop_assert_eq!(all.chars().count(), width);
        prop_assert!(all.chars().all(|c| c == ' ' || c.is_ascii_digit() || ('A'..='F').contains(&c)));
    }

    /// Any way of cutting the time into steps ends in the same place.
    #[test]
    fn the_size_of_the_steps_does_not_change_where_the_staging_ends(
        steps in proptest::collection::vec(1u64..400, 1..40),
    ) {
        let mut staging = laylow();
        for millis in &steps {
            staging.tick(Duration::from_millis(*millis));
        }
        let total: u64 = steps.iter().sum();
        let mut once = laylow();
        once.tick(Duration::from_millis(total));
        prop_assert_eq!(staging, once);
    }

    #[test]
    fn the_percentage_is_always_between_zero_and_a_hundred(millis in 0u64..10_000) {
        let mut staging = hack("localhost");
        staging.tick(Duration::from_millis(millis));
        prop_assert!(staging.percent() <= 100);
        prop_assert_eq!(staging.is_done(), staging.percent() == 100 || staging.progress() >= 1.0);
    }
}
