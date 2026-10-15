//! The optimistic player at the command level: it plays the real campaign through the plain
//! frontend contract (`Game::handle` with the lines and answers a frontend sends, using the
//! prompts the game gives), from a new game to the epilogue, and checks everything it sees.
//!
//! What to type and how to type it is the harness's (`neon_sim::brain` and
//! `neon_sim::driver`); this module is the observer that checks each step: both languages
//! render it, no text is a draft, errors are failures, and every save loads back identical.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use neon_engine::Game;
use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::content::Content;
use neon_engine::event::{Event, Role};
use neon_engine::prompt::{Prompt, Step};
use neon_engine::save::SaveRequest;
use neon_engine::text::{Arg, Catalog, Lang, RenderMode, Text, render};
use neon_sim::brain::Plan;
use neon_sim::driver::{Config, Driver, Observer};

/// What the walk does with the saves the game asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reload {
    /// Nothing: the game plays on.
    Never,
    /// At every checkpoint and slot, the game is saved and loaded back, and the loaded copy
    /// plays on (the walk must not notice).
    Checkpoints,
    /// The same at every save the game asks for, autosaves included: mid-menu too.
    EverySave,
}

fn catalogs() -> &'static Vec<(Lang, Catalog)> {
    static CATALOGS: OnceLock<Vec<(Lang, Catalog)>> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        Lang::ALL
            .iter()
            .map(|lang| (*lang, Catalog::embedded(*lang).unwrap()))
            .collect()
    })
}

/// FNV-1a: a stable hash, so a transcript hash means the same on every run.
fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// What a finished walk reports.
#[derive(Debug)]
pub(crate) struct Walked {
    pub(crate) steps: usize,
    /// FNV-1a of everything the game said (as data, so it does not depend on a language).
    pub(crate) hash: u64,
    pub(crate) checkpoints: usize,
    pub(crate) reloads: usize,
    /// The ending titles seen, in order.
    pub(crate) endings: Vec<String>,
    /// The epilogue lines seen, in order.
    pub(crate) epilogue: Vec<String>,
    /// How many times each quest was announced as completed.
    pub(crate) completed: BTreeMap<String, u32>,
    /// The credits announced as rewards.
    pub(crate) rewarded: i64,
    /// Every command line typed.
    pub(crate) typed: Vec<String>,
    /// Every line a plain frontend would read to send the same inputs (`y` for a yes, an
    /// empty line for "continue", `skip` for backing out): the script of the walk.
    pub(crate) script: Vec<String>,
    pub(crate) game: CampaignGame,
}

pub(crate) struct Player {
    plan: Plan,
    difficulty: Difficulty,
    checks: Checks,
}

impl Player {
    /// A new campaign. The renderings checked at every step are English and French in full
    /// mode, and with `thorough` also the screen-reader and ASCII modes.
    pub(crate) fn new(plan: &Plan, difficulty: Difficulty, reload: Reload, thorough: bool) -> Self {
        let mut modes = vec![RenderMode::FULL];
        if thorough {
            modes.extend([
                RenderMode::SCREEN_READER,
                RenderMode::ASCII,
                RenderMode {
                    screen_reader: true,
                    ascii: true,
                },
            ]);
        }
        Self {
            plan: plan.clone(),
            difficulty,
            checks: Checks {
                reload,
                modes,
                hash: 0xcbf2_9ce4_8422_2325,
                checkpoints: 0,
                reloads: 0,
                endings: Vec::new(),
                epilogue: Vec::new(),
                completed: BTreeMap::new(),
                rewarded: 0,
                last_said: String::new(),
            },
        }
    }

    /// Plays from a new game to the end of the campaign, then leaves.
    pub(crate) fn play(self) -> Walked {
        let game = CampaignGame::new(self.difficulty, 7).unwrap();
        let config = Config {
            slot_save_after: Some(40),
            ..Config::default()
        };
        let name = self.plan.name();
        let walk = Driver::new(game, self.plan, self.checks, config)
            .play()
            .unwrap_or_else(|stuck| panic!("plan [{name}]: {stuck}"));
        let checks = walk.observer;
        Walked {
            steps: walk.steps,
            hash: checks.hash,
            checkpoints: checks.checkpoints,
            reloads: checks.reloads,
            endings: checks.endings,
            epilogue: checks.epilogue,
            completed: checks.completed,
            rewarded: checks.rewarded,
            typed: walk.typed,
            script: walk.script,
            game: walk.game,
        }
    }
}

/// What is checked about each step of the walk.
struct Checks {
    reload: Reload,
    modes: Vec<RenderMode>,
    hash: u64,
    checkpoints: usize,
    reloads: usize,
    endings: Vec<String>,
    epilogue: Vec<String>,
    completed: BTreeMap<String, u32>,
    rewarded: i64,
    last_said: String,
}

impl Observer for Checks {
    fn observe(&mut self, game: &mut CampaignGame, what: &str, step: &Step) -> Result<(), String> {
        self.hash = fnv(self.hash, format!("{step:?}").as_bytes());
        self.last_said.clear();
        self.render_all(game, what, step)?;
        Self::check_drafts(what, step)?;
        self.tally(what, step)?;
        match step.save {
            Some(SaveRequest::Checkpoint) => {
                self.checkpoints += 1;
                self.round_trip(game, what, self.reload != Reload::Never)
            }
            Some(SaveRequest::Slot(_)) => self.round_trip(game, what, self.reload != Reload::Never),
            Some(SaveRequest::Autosave) => {
                self.round_trip(game, what, self.reload == Reload::EverySave)
            }
            None => Ok(()),
        }
    }

    fn context(&self) -> String {
        self.last_said.clone()
    }
}

impl Checks {
    /// Both languages render the step without a missing key or an argument left over.
    fn render_all(&mut self, game: &CampaignGame, what: &str, step: &Step) -> Result<(), String> {
        for (lang, catalog) in catalogs() {
            for mode in &self.modes {
                let mut said = String::new();
                for event in &step.events {
                    said.push_str(&render_event(event, catalog, *mode));
                }
                said.push_str(&render_prompt(&step.prompt, catalog, *mode));
                // The side panel reads the view: its sections and the gauge's band.
                let view = game.view();
                for text in view
                    .sections
                    .iter()
                    .flat_map(|section| {
                        std::iter::once(&section.title)
                            .chain(section.rows.iter().map(|row| &row.text))
                    })
                    .chain(view.gauges.iter().map(|gauge| &gauge.band))
                {
                    said.push_str(&render(text, catalog, *mode));
                }
                if said.contains("<missing:") || said.contains("<?") {
                    return Err(format!(
                        "`{what}` in {lang:?} {mode:?} has a missing key or argument:\n{said}"
                    ));
                }
                if *lang == Lang::En && *mode == RenderMode::FULL {
                    self.last_said = said;
                }
            }
        }
        Ok(())
    }

    /// Every text of the game is written: none of the keys a step says is still a `TODO` draft.
    fn check_drafts(what: &str, step: &Step) -> Result<(), String> {
        let mut keys = Vec::new();
        for text in step_texts(step) {
            collect_keys(text, &mut keys);
        }
        for key in keys {
            for (lang, catalog) in catalogs() {
                // The key alone: its own template, without what its arguments bring.
                let own = render(&Text::dynamic(key.clone()), catalog, RenderMode::FULL);
                if own.starts_with("TODO ") {
                    return Err(format!("`{what}`: `{key}` is still a draft in {lang:?}"));
                }
            }
        }
        Ok(())
    }

    /// Reads what the step said: errors are failures, the rest is counted.
    fn tally(&mut self, what: &str, step: &Step) -> Result<(), String> {
        for event in &step.events {
            let Event::Message { role, text, .. } = event else {
                continue;
            };
            let key = text.key.as_ref();
            match role {
                Role::Error => return Err(format!("`{what}` was refused: {key}")),
                Role::Reward if key == "campaign.out.credits" => {
                    if let Some((_, Arg::Int(amount))) =
                        text.args.iter().find(|(name, _)| *name == "amount")
                    {
                        self.rewarded += amount;
                    }
                }
                _ => {}
            }
            if matches!(
                key,
                "campaign.out.completed_quest" | "campaign.out.completed_contract"
            ) && let Some(title) = nested_key(text, "title")
            {
                *self.completed.entry(title).or_insert(0) += 1;
            }
            if key.starts_with("ending.") && key.rsplit('.').next() == Some("title") {
                self.endings.push(key.to_owned());
            }
            if key.starts_with("epilogue.") {
                self.epilogue.push(key.to_owned());
            }
        }
        Ok(())
    }

    /// The game is saved and loaded back: the copy must be the same game, and (when the walk
    /// asks for it) is the one that plays on.
    fn round_trip(
        &mut self,
        game: &mut CampaignGame,
        what: &str,
        swap: bool,
    ) -> Result<(), String> {
        let text = game.snapshot().unwrap();
        let loaded = CampaignGame::from_save(&text)
            .map_err(|e| format!("`{what}`: the save does not load: {e}"))?;
        if loaded.snapshot().unwrap() != text {
            return Err(format!("`{what}`: a loaded game saves differently"));
        }
        // A game that is over is not saved as over: a loaded game is never finished.
        let over = game.prompt() == Prompt::End;
        if (!over && loaded.prompt() != game.prompt()) || loaded.view() != game.view() {
            return Err(format!("`{what}`: a loaded game shows something else"));
        }
        if swap && !over {
            *game = loaded;
            self.reloads += 1;
        }
        Ok(())
    }
}

/// Every text of a step: its events and its prompt.
fn step_texts(step: &Step) -> Vec<&Text> {
    let mut texts = Vec::new();
    for event in &step.events {
        match event {
            Event::Message { text, .. } => texts.push(text),
            Event::Screen(table) => {
                texts.push(&table.title);
                texts.extend(&table.columns);
                texts.extend(table.rows.iter().flatten());
            }
            Event::Changed { band, .. } => texts.push(band),
            Event::Decor { alt, .. } => texts.push(alt),
            Event::Effect(effect) => texts.extend(effect.texts()),
            Event::Break => {}
        }
    }
    match &step.prompt {
        Prompt::Text { label, .. } => texts.push(label),
        Prompt::Confirm { question, .. } => texts.push(question),
        Prompt::Choice(choice) => {
            texts.push(&choice.title);
            for option in &choice.options {
                texts.push(&option.label);
                if let Err(reason) = &option.available {
                    texts.push(reason);
                }
            }
            texts.extend(&choice.cancel);
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
    texts
}

/// The key of a text and of every text nested in its arguments.
fn collect_keys(text: &Text, keys: &mut Vec<String>) {
    keys.push(text.key.to_string());
    for (_, arg) in &text.args {
        if let Arg::Text(inner) = arg {
            collect_keys(inner, keys);
        }
    }
}

/// The key of a text nested in an argument.
fn nested_key(text: &Text, name: &str) -> Option<String> {
    text.args.iter().find_map(|(arg, value)| match value {
        Arg::Text(inner) if *arg == name => Some(inner.key.to_string()),
        _ => None,
    })
}

/// An event as text, in a language and a mode.
fn render_event(event: &Event, catalog: &Catalog, mode: RenderMode) -> String {
    match event {
        Event::Message { text, .. } => format!("{}\n", render(text, catalog, mode)),
        Event::Screen(table) => {
            let mut out = format!("{}\n", render(&table.title, catalog, mode));
            for column in &table.columns {
                out.push_str(&render(column, catalog, mode));
                out.push('|');
            }
            for row in &table.rows {
                for cell in row {
                    out.push_str(&render(cell, catalog, mode));
                    out.push('|');
                }
                out.push('\n');
            }
            out
        }
        Event::Changed { band, .. } => format!("{}\n", render(band, catalog, mode)),
        Event::Decor { alt, .. } => format!("{}\n", render(alt, catalog, mode)),
        Event::Effect(_) => String::new(),
        Event::Break => "\n".to_owned(),
    }
}

/// A prompt as text: the question, the entries of a menu and the reasons they are closed.
fn render_prompt(prompt: &Prompt, catalog: &Catalog, mode: RenderMode) -> String {
    match prompt {
        Prompt::Command | Prompt::Continue | Prompt::End => String::new(),
        Prompt::Text { label, .. } => render(label, catalog, mode),
        Prompt::Confirm { question, .. } => render(question, catalog, mode),
        Prompt::Choice(choice) => {
            let mut out = render(&choice.title, catalog, mode);
            for option in &choice.options {
                out.push('\n');
                out.push_str(&render(&option.label, catalog, mode));
                if let Err(reason) = &option.available {
                    out.push_str(&render(reason, catalog, mode));
                }
            }
            if let Some(cancel) = &choice.cancel {
                out.push('\n');
                out.push_str(&render(cancel, catalog, mode));
            }
            out
        }
    }
}

/// The ending expected for a plan, from the rules of the game's story (decision D3, the echo
/// choice and the advantage of the echo log).
pub(crate) fn expected_ending(plan: &Plan) -> &'static str {
    let choice = |decision: &str| {
        plan.choices
            .iter()
            .find(|(d, _)| d.as_str() == decision)
            .map(|(_, c)| c.as_str())
    };
    match choice("d3") {
        Some("liberate") if matches!(choice("echo"), Some("wait" | "believe")) => "e1a",
        Some("liberate") => "e1b",
        Some("entrust") => "e2",
        Some("burn") => "e3",
        _ => "e4",
    }
}

/// The contacts that have an epilogue line, from the content.
pub(crate) fn epilogue_contacts(c: &Content) -> BTreeSet<String> {
    c.epilogue
        .iter()
        .map(|line| line.contact.to_string())
        .collect()
}
