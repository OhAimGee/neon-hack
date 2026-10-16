//! The drift of the nightly run: which indicator of a report moved by more than a given share
//! since the night before.
//!
//! The reports are canonical (no clock, no float, the same games give the same bytes), so two
//! nights over the same seeds and plans differ only because the game data or the rules did. The
//! comparison reads the numbers of two reports by their path (`intrusions.burned_per_mille`,
//! `commands.median`) and names those that moved. It is information, never a barrier: a change
//! of data is meant to move numbers, the alert says which.

use serde_json::Value;

/// The keys that say what was asked of the run, not what it measured: when one differs between
/// two nights the nights cannot be compared.
const PARAMETERS: [&str; 8] = [
    "kind",
    "schema_version",
    "bot",
    "difficulty",
    "seeds",
    "plans",
    "steps",
    "expansions",
];
/// The keys that hold games to replay, not indicators.
const SKIPPED: [&str; 1] = ["worst"];

/// An indicator that moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    /// Where the number is in the report: `intrusions.burned_per_mille`.
    pub path: String,
    /// The number the night before.
    pub previous: i128,
    /// The number tonight.
    pub current: i128,
}

impl Drift {
    /// The move in percent of the previous number (a number that was 0 moves by 100 % or more as
    /// soon as it is not 0), signed, rounded toward zero.
    #[must_use]
    pub fn percent(&self) -> i128 {
        (self.current - self.previous) * 100 / self.previous.abs().max(1)
    }
}

/// What two reports say of each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// The reports were made with other parameters: the key that differs.
    NotComparable(String),
    /// The reports can be compared: the indicators that moved by more than the share.
    Compared(Vec<Drift>),
}

/// Compares two reports: the indicators that moved by more than `percent` percent of their value
/// the night before, in the order of the report.
#[must_use]
pub fn compare(previous: &Value, current: &Value, percent: u64) -> Comparison {
    for key in PARAMETERS {
        if previous.get(key) != current.get(key) {
            return Comparison::NotComparable(key.to_owned());
        }
    }
    let mut numbers = Vec::new();
    collect(previous, String::new(), &mut numbers);
    let mut moved = Vec::new();
    for (path, before) in numbers {
        let Some(after) = number_at(current, &path) else {
            continue;
        };
        let delta = (after - before).abs();
        // `delta / |before| > percent / 100`, with 0 counting for 1 so that a number born
        // from nothing is a move.
        if delta * 100 > i128::from(percent) * before.abs().max(1) {
            moved.push(Drift {
                path,
                previous: before,
                current: after,
            });
        }
    }
    Comparison::Compared(moved)
}

/// The numbers of a report with their path, parameters and games to replay left out.
fn collect(value: &Value, path: String, out: &mut Vec<(String, i128)>) {
    match value {
        Value::Number(n) => {
            if let Some(number) = n
                .as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from))
            {
                out.push((path, number));
            }
        }
        Value::Object(map) => {
            for (key, inner) in map {
                if path.is_empty()
                    && (PARAMETERS.contains(&key.as_str()) || SKIPPED.contains(&key.as_str()))
                {
                    continue;
                }
                collect(inner, join(&path, key), out);
            }
        }
        Value::Array(items) => {
            for (index, inner) in items.iter().enumerate() {
                collect(inner, join(&path, &index.to_string()), out);
            }
        }
        Value::Null | Value::Bool(_) | Value::String(_) => {}
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// The number at a path of a report, if there is one.
fn number_at(value: &Value, path: &str) -> Option<i128> {
    let mut node = value;
    for part in path.split('.') {
        node = match node {
            Value::Array(items) => items.get(part.parse::<usize>().ok()?)?,
            other => other.get(part)?,
        };
    }
    let n = node.as_number()?;
    n.as_i64()
        .map(i128::from)
        .or_else(|| n.as_u64().map(i128::from))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use serde_json::json;

    use super::*;

    fn report(median: u64, burned: u64) -> Value {
        json!({
            "kind": "campaign",
            "schema_version": 5,
            "bot": "prudent",
            "difficulty": "normal",
            "seeds": [1, 3],
            "plans": 8,
            "games": 24,
            "commands": { "min": 600, "median": median },
            "intrusions": { "total": 540, "burned_per_mille": burned },
            "worst": [{ "seed": 9, "plan": "p", "why": "w" }],
        })
    }

    #[test]
    fn the_same_report_does_not_drift() {
        let r = report(622, 100);
        assert_eq!(compare(&r, &r, 5), Comparison::Compared(Vec::new()));
    }

    #[test]
    fn a_number_that_moved_by_more_than_the_share_is_named_with_its_path() {
        let Comparison::Compared(moved) = compare(&report(1000, 100), &report(1049, 106), 5) else {
            panic!("the reports can be compared");
        };
        // 4.9 % is under the share, 6 % is over.
        assert_eq!(moved.len(), 1, "{moved:?}");
        let drift = moved.first().unwrap();
        assert_eq!(drift.path, "intrusions.burned_per_mille");
        assert_eq!(
            (drift.previous, drift.current, drift.percent()),
            (100, 106, 6)
        );
        // Exactly the share is not over it.
        let Comparison::Compared(moved) = compare(&report(1000, 100), &report(1050, 105), 5) else {
            panic!("the reports can be compared");
        };
        assert!(moved.is_empty(), "{moved:?}");
    }

    #[test]
    fn a_number_born_from_nothing_and_one_that_falls_are_moves() {
        let Comparison::Compared(moved) = compare(&report(600, 0), &report(300, 1), 5) else {
            panic!("the reports can be compared");
        };
        let paths: Vec<(&str, i128)> = moved
            .iter()
            .map(|d| (d.path.as_str(), d.percent()))
            .collect();
        assert_eq!(
            paths,
            [
                ("commands.median", -50),
                ("intrusions.burned_per_mille", 100)
            ]
        );
    }

    #[test]
    fn the_games_to_replay_are_not_indicators() {
        let mut moved = report(622, 100);
        moved["worst"] = json!([{ "seed": 90, "plan": "q", "why": "x" }]);
        assert_eq!(
            compare(&report(622, 100), &moved, 5),
            Comparison::Compared(Vec::new())
        );
    }

    #[test]
    fn two_runs_over_other_seeds_cannot_be_compared() {
        let mut other = report(622, 100);
        other["seeds"] = json!([1, 20]);
        assert_eq!(
            compare(&report(622, 100), &other, 5),
            Comparison::NotComparable("seeds".to_owned())
        );
    }
}
