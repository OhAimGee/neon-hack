# Architecture du moteur Rust : conception et spike vérifié

> Livrable de la tâche D (architecture du moteur pour la réécriture en Rust).
> Dépôt lu : branche `claude/relaxed-lamport-87hbbr` ; aucun fichier suivi n'a été modifié, ce fichier est le seul ajout de cette tâche (`docs/design/` contient aussi les livrables des autres tâches).
> Spike (hors dépôt, jetable) : `<scratchpad>/spike-engine`. Les extraits de code de ce document en viennent (abrégés par `…`) ; si le répertoire disparaît, rien d'essentiel n'est perdu ici.
> Les identifiants, noms de crates et extraits de code restent en anglais ; le reste est en français.

**Légende des preuves** : `[E]` exécuté dans cette session (commande lancée, résultat lu) ; `[C]` lu dans le code du dépôt ou dans la source d'une crate ; `[H]` hypothèse, estimation ou raisonnement non vérifié.

## 0. Résumé

1. **API recommandée : une machine à états pure**, `Engine::handle(Input) -> Outcome { output, prompt, dispatch, save_requested }`. Menus, boutique, conversations et prologue ne lisent jamais : ce sont des états empilés (`Vec<Flow>`), `Clone`, sérialisables, dérivés dans un `Prompt`. Le C a 12 sites de lecture bloquante dans `src/game/*.c` `[C]` ; le spike les remplace par 3 flux et une fonction `pick` commune `[E]`.
2. **Écartés** : le `trait Ui` à lecture bloquante (le moteur reste maître de la boucle, rien à cloner ni à sauvegarder en plein menu), la coroutine (`gen` est instable sur la 1.97, un `async` n'est ni `Clone` ni `Serialize`, `[E]`) et les continuations par fermetures (même défaut).
3. **Le moteur n'a ni horloge, ni E/S, ni globale, ni `HashMap`** : c'est imposé par `clippy` (liste de types et d'appels interdits ; `[E]` : `println!`, `HashMap`, `SystemTime`, `fs::read_to_string`, `thread::sleep`, `env::var`, `unsafe`, `unwrap` et l'indexation se déclenchent sur une violation volontaire). Le temps de jeu est un compteur de tours, la graine entre par le constructeur, le PCG32 est **identique bit à bit au C** (7 suites de valeurs de référence calculées par le vrai `rng.c` compilé avec `gcc`, `[E]`).
4. **Contenu : TOML embarqué** (structure en `data/world/`, textes en `data/text/<langue>/`), parsé à chaque démarrage (7,6 ms pour ≈ 1 500 clés × 2 langues, release, `[E]`) et validé en tests : références, doublons, prérequis acycliques, joueur optimiste (« pas d'impasse »), parité FR/EN des clés **et des marqueurs**, budgets en largeur d'affichage, caractères interdits. Chaque contrôle est testé en cassant volontairement une copie des données `[E]`.
5. **i18n maison, au rendu** : le moteur n'émet que des `Text { clé, arguments typés }` ; les messages système passent par une macro `messages!` (clé et marqueurs autorisés déclarés une seule fois), le contenu par des familles de clés dérivées de la structure. Pluriels `one`/`other` avec règle CLDR par langue (0 est singulier en français, pas en anglais, `[E]`).
6. **Sauvegarde : TOML versionné**, migrations sur l'arbre non typé, marqueur de fin `[end]` (**tous** les préfixes d'une sauvegarde sont rejetés, plus de 1 000 coupes testées, `[E]`), chargement transactionnel (on construit un *nouvel* `Engine`), écriture atomique `tempfile` + `fsync` + renommage, 9 emplacements avec copie `.bak` et mise en quarantaine `.corrupt`.
7. **Tests : 94, tous verts en 6 s** `[E]` : 15 unitaires, 16 de contenu, 18 de flux, 10 `proptest` (9 propriétés et un garde-fou contre le test vide), 9 de sauvegarde, 26 côté CLI (dont 2 transcripts `insta` FR et EN). 13 mutants injectés à la main : 11 attrapés, 2 équivalents (garde redondante) ; **5 survivants du premier passage ont produit 2 tests de plus**, un troisième mutant n'étant attrapé qu'en exécutant aussi la CLI `[E]`.
8. **Chiffres du spike** `[E]` : 3 721 lignes de moteur, 1 319 de tests moteur, 732 de CLI, 365 de tests CLI, 457 de données ; `rustc 1.97.0` et `1.88.0` (MSRV vérifié), `clippy -D warnings` propre, `cargo check` sur `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu` et `aarch64-apple-darwin` ; 1,1 à 1,4 million de commandes par seconde (release, 4 cœurs), 447 ns par `Engine::clone`.
9. **Frictions réelles** (§ 9.4) : `toml` 1.1.6 écrit un `u64` au-delà de `i64::MAX` mais ne le relit pas dans un arbre non typé ; `std::env::set_var` est `unsafe` en édition 2024 ; `allow-unwrap-in-tests` de clippy ne couvre pas les fonctions d'aide des tests d'intégration ; une génération d'entrées purement aléatoire n'atteint la boutique que dans 0,3 % des parties ; **un test de ma main a écrit dans le vrai dossier de données** avant que l'environnement ne soit rendu hermétique.
10. **8 décisions à valider** (§ 10).

---

## 1. Point de départ : ce que le C impose (lu)

Faits relevés dans `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `src/game/*.h`, `tests/unit/nh_*.h` et le code `[C]`, sauf mention.

| Mécanisme du C | Où | Ce que le Rust garde ou change |
|---|---|---|
| `GameState` unique, plus de globale d'état | `game.h` | `Engine { content: Arc<Content>, world: World, bus: Bus, flows: Vec<Flow>, ended }`. Seul `World` est sauvegardé : la file d'événements et la pile de flux sont transitoires **par construction** (types séparés), plus par convention. |
| Table de commandes = source de vérité, 29 entrées, une cachée | `commands.c:149-183` | Macro `commands!` : une déclaration produit l'enum, la fiche et `ALL`. Le handler est un `match` exhaustif (pas de pointeur de fonction). |
| Bus différé : file de 32, 256 livraisons, abonnés qui relisent l'état, jamais sauvegardé | `events.h:52`, `events.c:7`, `ARCHITECTURE.md` | Même sémantique, mêmes bornes, en réacteurs `fn(&mut Ctx, &Event)` d'ordre fixe (§ 4.3). |
| Quêtes en tables `static const`, objectifs « à niveau » (mesurés sur l'état), statut posé avant de payer | `quest_system.h/.c` | `[[quest]]` en TOML ; `quests::measure`, `complete`, `refresh` idempotent. |
| Anti-farm : toute source bornée par un état ou un jalon à usage unique | `ARCHITECTURE.md`, `test_economy.c` | Registre `World::rewards` de `RewardId`, unique point de paiement `claim`, propriété `proptest` (§ 8). |
| Sauvegarde : version en tête, `end=1`, chargement transactionnel, écriture atomique, `NH_SAVE_MAX_BYTES` 256 Kio | `save.h:28-29`, `storage.h` | Même contrat, en TOML (§ 7). |
| i18n : `strings.def`, 436 textes, parité des marqueurs `printf` | `strings.def`, `test_i18n.c` | Catalogues TOML, parité des clés, des marqueurs et des formes plurielles (§ 6). Le contenu prévu est ≈ 1 300 chaînes par langue (bible narrative), soit 3 fois plus. |
| **12 lectures bloquantes** : `intro.c` ×3, `menu.c` ×2, `game.c` ×1, `cmd_world.c` ×2, `contacts.c` ×2, `cmd_advanced.c` ×1, `quest_system.c` ×1 | `grep nh_read_line\|nh_lineedit_read` | Flux explicites (§ 3). |
| Sentinelles et index : `NH_NO_UNLOCK (-1)`, `prerequisites[2]` avec `-1`, `NhObjectiveDef.arg` surchargé (`-1` = tous les systèmes), `commands_unlocked[MAX_COMMANDS]` indexé par `CMD_*`, valeurs d'enum écrites dans les sauvegardes (« ne pas les réordonner ») | `commands.h:12`, `quest_system.h:81,94`, `game_types.h:87`, `contacts.h:34,57` | Identifiants textuels stables, `Option`, variantes d'enum distinctes (§ 4.1). |
| RNG : PCG32 | `rng.c` | Réécrit à l'identique, vérifié contre le C (§ 3.4). |
| Priorité des réglages : environnement < fichier < CLI, avec l'exception `NO_COLOR` | `ARCHITECTURE.md` | Règle **par réglage**, testée (§ 7.6). |
| Code Windows jamais compilé | `storage.h:12` | `cargo check` Windows et macOS en intégration continue dès le premier lot (§ 7.4). |

Ce que le C ne dit pas et que cette conception doit trancher : comment sauvegarder en plein menu (le C ne le peut pas : l'état est dans la pile d'appels), comment cloner une partie pour un `undo` ou un solveur (l'audit propose les deux), comment rejouer le même flux de sortie en deux langues.

---

## 2. Workspace, crates, toolchain, lints, erreurs (point 1)

### 2.1 Découpage

```
Cargo.toml                 workspace : resolver 3, édition 2024, rust-version 1.88, [workspace.lints], [workspace.dependencies]
rustfmt.toml               edition 2024, max_width 100, use_small_heuristics "Max"
crates/neon-engine/        lib, PURE : règles, état, événements, RNG, contenu, i18n, sérialisation
    build.rs               embarque data/**/*.toml (aucune dépendance)
    clippy.toml            interdits du moteur (HashMap, horloge, fichiers, stdin, sleep, env)
crates/neon-cli/           binaire `neon-hack` : clap, répertoires, stockage, frontend plain, frontend TUI (feature `tui`)
crates/neon-sim/           harnais d'équilibrage (à écrire, absent du spike) : dépend de neon-engine seul
data/world/*.toml          structure du jeu, sans texte
data/text/{fr,en}/*.toml   textes, mêmes clés
fuzz/                      cargo-fuzz (nightly), hors workspace [H]
```

Règle de dépendance : `neon-cli → neon-engine` et `neon-sim → neon-engine`, jamais l'inverse. `neon-engine` n'a aucune dépendance terminal, fichier ou temps.

| Crate | Dépendances directes | Crates dans l'arbre, elle-même comprise (hors dev) `[E]` |
|---|---|---|
| `neon-engine` | `serde`, `toml`, `thiserror`, `unicode-width` | 20 (dont 4 proc-macros) |
| `neon-cli` (plain) | + `clap`, `directories`, `tempfile`, `anyhow` | 50 |
| `neon-cli` + `tui` | + `ratatui` | 117 |

Le spike n'a pas de `neon-sim` : le moteur lui offre ce qu'il lui faut (`Engine: Clone`, `complete`, `prompt`, déterminisme) et le débit mesuré (§ 9.3) montre que 10⁵ parties tiennent en secondes. **API à ajouter** (non faite) : `Engine::legal_inputs() -> Vec<Input>`. Le spike la reconstruit dans les tests à partir de `prompt()` et `complete()` (`valid_inputs`, `tests/props.rs`) : elle sert aux propriétés, aux robots de `neon-sim` et à une commande d'aide accessible (« que puis-je faire maintenant ? »).

### 2.2 Features

- `neon-cli/tui` : écran plein. **Défaut recommandé : activé** dans les binaires publiés (priorité 2 du propriétaire), désactivable (`--no-default-features`) pour un binaire plain minimal (50 crates au lieu de 117). Le spike le laisse désactivé par défaut pour compiler vite ; la CI doit construire les deux `[H]`.
- `neon-engine` : aucune feature. Les besoins de test (trace du bus, `legal_inputs`) sont de l'API publique, pas du `cfg(test)` invisible depuis `neon-sim`.
- Séparer `neon-tui` en crate : écarté pour l'instant (l'audit estime la TUI à 2-3 k lignes, une feature suffit) ; à reconsidérer si elle dépasse 3 k lignes `[H]`.

### 2.3 Édition, MSRV, résolveur

- **Édition 2024, résolveur 3, `rust-version = "1.88"`.** Le plancher réel est 1.88 : `ratatui 0.30.2` et `darling` le déclarent, c'est le maximum du graphe résolu (228 paquets dans `Cargo.lock`, dont 75 sans `rust-version` déclaré) `[E]` ; le code utilise les chaînes `if let … && …` (stables en 1.88, édition 2024). **Vérifié** : `cargo +1.88.0 test --workspace --features neon-cli/tui` passe les 94 tests `[E]`.
- Le résolveur sensible à la MSRV a verrouillé des versions « compatibles Rust 1.88 » (message de `cargo`) ; seul `uuid` a été retenu en arrière (1.26.1, la 1.27.0 exige 1.89) `[E]`. Toutes les crates directes sont à leur dernière version.
- Politique : monter la MSRV seulement quand une dépendance l'exige (aujourd'hui `ratatui`), jamais pour une commodité du code ; un job CI sur la MSRV `[H]`.

### 2.4 Lints

```toml
[workspace.lints.rust]
unsafe_code = "forbid"                       # moteur et CLI
missing_debug_implementations = "warn"
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 } # + 7 exceptions : module_name_repetitions, missing_errors_doc,
                                             #   missing_panics_doc, must_use_candidate, return_self_not_must_use,
                                             #   similar_names, too_many_lines
unwrap_used = "warn"  expect_used = "warn"  indexing_slicing = "warn"   # entrée hostile : jamais de panique
dbg_macro = "deny"  todo = "deny"  unimplemented = "deny"
disallowed_types = "deny"  disallowed_methods = "deny"                    # pureté du moteur
```

Le moteur ajoute `#![deny(clippy::print_stdout, clippy::print_stderr)]` et `crates/neon-engine/clippy.toml` (`HashMap`, `HashSet`, `SystemTime`, `Instant`, `fs::read_to_string`, `fs::write`, `File::open`, `File::create`, `io::stdin`, `thread::sleep`, `env::var`).

**Preuve que ces interdits fonctionnent** `[E]` : une copie du moteur avec `println!`, `HashMap`, `SystemTime::now`, `fs::read_to_string`, `thread::sleep`, `env::var`, un indexage, un `unwrap` et un bloc `unsafe` donne 1 erreur `unsafe`, 1 erreur `println!` et toutes les autres violations signalées. `HashMap` est interdit parce que son ordre d'itération change à chaque processus : il casserait le déterminisme des transcripts. `BTreeMap`/`BTreeSet` partout. Interdits **non exercés** individuellement : `HashSet`, `File::open`, `File::create`, `io::stdin` (même mécanisme).

Résultat du spike : `cargo clippy --workspace --all-targets --features neon-cli/tui -- -D warnings` sans aucun avertissement (trois `#[allow]` locaux commentés : `needless_pass_by_value` sur `Engine::handle`, qui consomme son message comme un récepteur de canal ; `trivially_copy_pass_by_ref` sur la signature imposée par `serde(with)` ; `struct_excessive_bools` sur la structure d'options `clap`), `cargo fmt --check` propre `[E]`. Le premier passage avait ≈ 160 avertissements `unwrap`/`expect` dans les tests et 7 dans `build.rs` ; la parade est un `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` en tête de chaque fichier de test d'intégration (la clé `allow-unwrap-in-tests` de `clippy.toml` ne couvre **pas** les fonctions d'aide non marquées `#[test]`, `[E]`) et d'un `#![allow(clippy::disallowed_methods)]` dans `build.rs`.

### 2.5 Erreurs

| Couche | Outil | Types |
|---|---|---|
| `neon-engine` | `thiserror 2.0.21`, erreurs typées | `LoadError` (contenu), `ContentError` (validation, liste complète), `CatalogError`, `TemplateError`, `IdError`, `SaveError` (`Syntax`, `MissingVersion`, `TooNew`, `Unsupported`, `Truncated`, `Schema`, `Migration`, `Invalid`) |
| `neon-cli::storage` | erreurs typées | `LoadError { Missing, Io, Save }` |
| `neon-cli::app` et `main` | `anyhow 1.0.104`, **seulement ici** | contexte (« the embedded game content is invalid », « slot 1 cannot be loaded… ») |

Règle : une entrée hostile (saisie, fichier, octets) donne une erreur ou un message, jamais une panique ni une boucle ; `Input::Eof` termine la partie. Les erreurs de validation de contenu sont renvoyées **toutes ensemble** (un rédacteur corrige un fichier par exécution, pas une erreur par exécution).

### 2.6 Politique de dépendances

Curée : 4 crates pour le moteur, une par besoin. Toute crate ajoutée au moteur doit justifier qu'elle n'apporte ni E/S, ni horloge, ni état global. Les crates non compilées ici (`fluent-bundle`, `rust-i18n`, `rand`, `rand_pcg`, `rust-embed`, `atomic-write-file`, `etcetera`, `dirs`) n'ont été examinées que par `cargo info` ou lecture de source (§ 9.2).

### 2.7 Cohérence avec `delivery-and-ci.md`

Le document de livraison arrive indépendamment aux mêmes choix : espace de travail `neon-engine` / `neon-cli` / `neon-sim`, édition 2024, résolveur 3, MSRV 1.88 imposée par `ratatui 0.30.2`, interdits `clippy` du moteur (horloge, environnement, disque, console, `HashMap`). Deux écarts à aligner : `tempfile` est ici une **dépendance d'exécution** de `neon-cli` (écriture atomique des sauvegardes, § 7.4) alors que ce document la cite en dev-dependency ; et `ron`, présent dans son « arbre complet », n'est **pas retenu** (§ 5.1) : il ne sert qu'à la sonde de formats.

---

## 3. API du moteur (point 2)

### 3.1 Contrat (extrait du spike, compilé)

```rust
pub enum Input  { Line(String), Choice(usize), Continue, Cancel, Eof }

pub enum Prompt {
    Command,                                              // ligne de commande (TAB via Engine::complete)
    Menu { title: Text, items: Vec<MenuItem> },           // réponse par numéro OU par id stable
    Line { label: Text, default: Option<String> },
    Confirm { question: Text, default: bool },
    Continue,                                             // « Entrée pour continuer » : remplace toute pause chronométrée
    End,
}
pub struct MenuItem { pub id: String, pub label: Text, pub enabled: bool }

pub enum Output {
    Line(Text),                                           // résultat de la commande
    Notice(Text),                                         // annonce après coup (quête, niveau, contact)
    Say { speaker: ContactId, line: Text },               // toujours rendu « NOM: réplique » (pas de couleur seule)
    Help(Vec<HelpEntry>),
}
pub struct Outcome { pub output: Vec<Output>, pub prompt: Prompt, pub dispatch: Option<Dispatch>, pub save_requested: bool }

impl Engine {
    pub fn new_game(content: Arc<Content>, seed: u64) -> (Engine, Outcome);
    pub fn handle(&mut self, input: Input) -> Outcome;    // la seule façon de faire avancer le jeu
    pub fn prompt(&self) -> Prompt;                       // dérivé de l'état : valable aussi juste après un chargement
    pub fn complete(&self, line: &str) -> Vec<String>;    // TAB, sans spoiler
    pub fn view(&self) -> View;                           // lecture seule pour le panneau latéral
    pub fn to_toml(&self) -> Result<String, toml::ser::Error>;
    pub fn from_toml(content: Arc<Content>, text: &str) -> Result<Engine, SaveError>;
}
```

`Text` est `{ key, args: Vec<(&'static str, Arg)> }` avec `Arg::{Int, Str, Text(Box<Text>)}` : un nom d'objet ou de contact est un `Text` imbriqué rendu **dans la langue du rendu**. Le moteur ne produit jamais une phrase.

### 3.2 Comparaison des styles

| Critère | (a) `commande -> Vec<Output>` seul | (b1) canal / `trait Ui` à lecture bloquante | (b2) effets en données | (c) coroutine / `async` | **(a′) machine à états + pile de flux (recommandé)** |
|---|---|---|---|---|---|
| Menus imbriqués | impossible sans état : il faut un moyen de « reprendre » | naturel (code séquentiel) | idem (a) | naturel | pile de `Flow`, `Next::{Stay, Pop, Push}` |
| Sauvegarde en plein menu | non | non (l'état est dans la pile d'appels) | oui si l'état est de la donnée | **non** : `async` n'est pas `Serialize` `[E]` | **oui** (`kind = "conversation"` dans `[[flows]]`, rechargé et repris, `[E]`) |
| `undo`, solveur, `proptest` (cloner) | oui | non | oui | **non** : un futur n'est pas `Clone` `[E]` ; `gen` instable `[E]` | **oui**, 447 ns par `Engine::clone` `[E]` |
| Maître de la boucle (TUI événementielle) | le frontend | le moteur (il faut un fil + canaux) | le frontend | l'exécuteur | **le frontend** (`handle` à chaque touche) |
| Tests déterministes sans fil | oui | scripter un `Ui` factice | oui | exécuteur maison | oui : un transcript est une liste de `Input` |
| Pause chronométrée dans le moteur | n/a | tentant (`sleep`) | non | tentant | **interdit** : `Prompt::Continue` |
| Emprunts (`&mut World` à travers les attentes) | simple | simple | simple | **pénible** (`Rc<RefCell>` ou passer le contexte à chaque reprise) | simple (`Ctx` par appel) |
| Texte passe-partout par flux | nul | faible | moyen | faible | moyen : 307 lignes pour 3 flux et 4 étapes de prologue (`flows.rs`) |

`[E]` : `gen { yield 1 }` donne `E0658 gen blocks are experimental` sur `rustc 1.97.0` en édition 2024 ; `assert_clone(&async_fn_future)` et `assert_serialize` échouent (`E0277`). `[H]` : les bibliothèques de coroutines tierces et le style continuation (`Box<dyn FnOnce>`) ont le même défaut (ni `Clone` ni sérialisables).

**Recommandation : (a′)**, qui est (a) étendu par une pile de flux et complété par (b2) pour les requêtes d'E/S (`save_requested` : le moteur *demande*, le frontend écrit et dit si cela a réussi). Raisons : les trois priorités du propriétaire (campagne complète : tout est de la donnée testable ; TUI : le frontend garde la boucle ; accessibilité : aucune pause, chaque état a un `Prompt` énonçable) et trois besoins de l'audit (`undo`, solveur de mission, reprise en plein dialogue). Le coût est le texte passe-partout d'un flux.

### 3.3 Flux imbriqués

Un flux est une variante d'enum sérialisable ; `prompt(flow, world, content)` dit ce qu'il attend, `step(flow, ctx, input) -> Next` consomme une entrée :

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Flow {
    Prologue { step: PrologueStep, name: String },   // Intro (Continue) -> Name -> Confirm -> Tutorial
    Conversation { contact: ContactId },
    Shop { seller: ContactId },
    // à venir : Run { state: RunState }, Scene { id, paragraph }
}
```

Parcours vérifié `[E]` (`nested_menus_are_states_not_blocking_reads`) : `contact r4z0r` → menu de conversation → `wares` (par id) → menu de boutique → achat (la boutique reste ouverte) → `Cancel` → conversation → `Cancel` → invite de commandes. Règles communes de `pick` : numéro à partir de 1, id stable insensible à la casse, `0`/ligne vide/`Cancel` quittent, `Input::Choice(i)` direct pour la TUI, saisie invalide expliquée sans rien changer. Toute fin d'entrée (`Eof`) à n'importe quelle profondeur termine la partie ; une partie terminée ne répond plus (`prompts_are_coherent_and_end_is_final`).

**Les conversations sont de la donnée, pas du code** : `contacts.toml` déclare des sujets `{ id, effect = say|mission|shop, when = [conditions] }`, et un seul flux `Conversation` les interprète. Ainsi les 9 contacts et leurs conditions (exigences de `narrative-bible.md` § 5.6) ne coûtent aucune ligne de moteur par contact. Seuls les flux aux règles propres (boutique, prologue, future intrusion) sont du code.

**Où se place la mécanique de piratage** : la tâche A recommande des intrusions à tours, avec `AutoResolve` d'abord. L'architecture n'en dépend pas : une intrusion est un `Flow::Run` (état sérialisable et `Clone`, donc `undo` = pile d'instantanés et solveur = recherche sur des états clonés), derrière une interface à deux implémentations. Rien dans le moteur actuel ne doit changer pour l'accueillir.

### 3.4 Déterminisme, RNG, horloge

- **Pas d'horloge dans le moteur.** Le temps de jeu est `World::turn` ; la date d'une sauvegarde est celle du fichier (mtime), lue côté CLI ; la graine par défaut est tirée de l'horloge par `main` et passée comme un nombre. Si une fonctionnalité exigeait l'heure (contrat quotidien, durée de jeu), elle entrerait comme **valeur d'un `Input`**, jamais par un appel système, donc rejouable. Cela diffère de la consigne « Clock injectée » : un trait `Clock` injecté resterait un accès au temps réel, et l'invariant I22 de l'audit demande un moteur sans horloge `[C]`.
- **Le RNG fait partie de l'état** (`World::rng`, sérialisé) : un jeu rechargé continue exactement comme l'original (`round_trip_is_exact` compare les sorties des commandes suivantes, `[E]`). `Engine<R: Rng>` générique : écarté, il alourdit toute l'API pour un besoin que les graines couvrent `[H]`.
- **PCG32 maison, identique au C** : `rand` a changé l'échantillonnage d'entiers d'une version majeure à l'autre `[H]`, ce qui casserait transcripts et graines ; l'algorithme (30 lignes) est figé. Vérifié contre le vrai `src/core/rng.c` compilé avec `gcc` : vecteur de référence PCG `(42, 54)` (`0xa15c02b7, 0x7b47f409, …`), plages étroites, plages larges (≈ 30 % de rejets, qui exercent le seuil anti-biais) et plage pleine `i32::MIN..=i32::MAX` `[E]`. Un mutant qui supprime le seuil n'est attrapé que par les plages larges : d'où ce test.
- **Hachage d'état stable** : FNV-1a (10 lignes) sur le TOML canonique, pas `DefaultHasher`, dont l'algorithme n'est pas garanti d'une version de Rust à l'autre `[H]`.

Débit `[E]` (release, 4 cœurs, état de la taille du spike) : 1,14 à 1,39 million de commandes par seconde (20 000 parties de 13 commandes en 0,19-0,23 s) ; `Engine::clone` 447 ns. Avec le contenu complet, compter 3 à 5 fois plus de coût de clone `[H]`.

### 3.5 La boucle côté frontend (plain)

```rust
ui.render(out, first)?;
while !engine.is_over() {
    let outcome = engine.handle(read_input(input)?);   // octets invalides -> U+FFFD, EOF -> Input::Eof
    ui.render(out, &outcome)?;
    if outcome.save_requested || (outcome.dispatch == Some(Dispatch::Ok) && !engine.is_over()) { persist(&engine)… }
}
```

40 lignes (`plain.rs`) ; la TUI fait la même chose dans sa boucle d'événements, sans fil. Les annonces (`Notice`) portent un marqueur textuel (`* `) : rien ne dépend de la couleur.

### 3.6 Cohérence avec `docs/design/tui-and-accessibility.md`

Ce document définit le même contrat sous d'autres noms : `Step { events, prompt }`, `Event::{Message{role, importance, text}, Decor, Screen(Table), Trace, Break, Quit}`, `Prompt::{Command, Choice, Text, Confirm, Continue}`, `Input::{Line, Choice, Confirm, Continue}`. Son spike n'a **pas** câblé les menus (« `Prompt`, `Input` et `Step` compilent mais ne sont pas câblés », § 2.2 du document TUI) : le présent spike comble ce trou et valide la mécanique. À faire converger avant d'écrire le vrai moteur (décision technique, pas propriétaire) :

| Sujet | Ce spike | Document TUI | Proposition |
|---|---|---|---|
| Sortie | `Output`, `Outcome` | `Event`, `Step` | adopter `Event`/`Step` ; renommer le bus interne en `Fact` pour éviter la collision |
| Rôles | `Line`, `Notice`, `Say`, `Help` | `Role`, `Importance`, `Decor`, `Screen(Table)`, `Trace` | adopter les leurs (besoins TUI non spikés ici) |
| Pause | `Prompt::Continue`, `Input::Continue` | idem | identique |
| Pluriels | table `{ one, other }` | inline `{n\|un nœud\|{n} nœuds}` | un seul parseur : l'inline gère plusieurs nombres par phrase, la table se valide mieux par forme ; trancher à l'écriture du catalogue |
| Variantes de rendu | non | `clé@sr` (lecteur d'écran), `--ascii` | recherche `clé@mode` puis `clé` : ~5 lignes dans `Catalog` `[H]` |

---

## 4. Types de base (point 3)

### 4.1 Identifiants et fin des sentinelles

Six types distincts (`QuestId`, `SiteId`, `ContactId`, `TopicId`, `ItemId`, `RewardId`) générés par une macro d'une quarantaine de lignes : `Arc<str>` (clone bon marché), `[a-z0-9_-]{1,48}` validé **à la désérialisation** (64 pour `RewardId`, qui préfixe un identifiant de contenu). Un `QuestId` ne peut pas être passé là où un `SiteId` est attendu. Ils sont **textuels et stables** dans les données et les sauvegardes : réordonner un tableau ne corrompt plus rien.

| C | Rust |
|---|---|
| `NH_NO_UNLOCK (-1)` + `min_level` | `enum Unlock { Always, Tier(u8) }` |
| `prerequisites[2]`, `-1` = aucune | `Vec<QuestId>` (vide) |
| `arg = -1` = « tous les systèmes », `target` surchargé selon le type d'objectif | une variante d'`ObjectiveDef` par sens : `Compromise { site }`, `Buy { any_of }`, `Reputation { min }`… |
| `commands_unlocked[MAX_COMMANDS]` indexé par `CMD_*` | **déduit** : `is_available(world, content, id)` ; rien à stocker ni à désynchroniser |
| `nh_contact_find` renvoie `-1` | `Option<ContactId>` |
| valeurs d'enum écrites dans la sauvegarde | chaînes d'id ; un renommage exige une migration, il ne passe plus inaperçu |
| niveau stocké et dérivé de l'XP | **dérivé seulement** (`progression.tier_for(xp)`) : un invariant de moins |
| `QuestStatus::{AVAILABLE, FAILED}` « réservés » | variantes ajoutées le jour où elles sont produites |

Non fait dans le spike `[H]` : les montants sont des `u32`/`i32` nus avec **un seul point d'entrée par grandeur** (`earn_credits`, `grant_xp`, `grant_reputation`, bornés par `credits_cap`, `xp_cap`, ±10⁶). Le vrai moteur doit les envelopper dans des newtypes saturants (`Credits`, `Xp`, `Reputation`) pour que l'arithmétique non bornée ne compile pas.

### 4.2 Registre de commandes

```rust
commands! {
    Scan     { name: "scan",     aliases: [],          category: Hack,   unlock: Unlock::Always,  help: "help.scan",    arg: None }
    Hack     { name: "hack",     aliases: ["breach"],  category: Hack,   unlock: Unlock::Always,  help: "help.hack",    arg: Site }
    Exploit  { name: "exploit",  aliases: [],          category: Hack,   unlock: Unlock::Tier(2), help: "help.exploit", arg: Site }
    Contact  { name: "contact",  aliases: [],          category: World,  unlock: Unlock::Always,  help: "help.contact", arg: Contact }
    …
}
```

La macro génère `enum CommandId`, `CommandId::ALL` et `const fn spec(self) -> &'static CommandSpec` : l'enum et la table sont **la même déclaration**. `run(id, ctx, arg)` est un `match` sans joker : une commande sans handler ne compile pas (à la place de la table de pointeurs de fonction du C). Les clés d'aide ne sont pas des variantes d'un enum de textes : `required_texts` les déduit de la table, donc une commande sans `help.<nom>` dans les deux langues fait échouer `shipped_content_is_valid`.

Testé `[E]` : noms et alias uniques et en minuscules, recherche insensible à la casse ; l'aide liste **exactement** les commandes exécutables ; une commande verrouillée répond `Locked` (≠ `Unknown`) en disant à quel niveau elle s'ouvre ; la complétion ne propose ni commande verrouillée, ni système non découvert, ni contact verrouillé, et un alias n'est proposé que si aucun nom officiel ne convient (règle du C).

### 4.3 Bus d'événements

Gardé du C : livraison **différée** (émettre n'enregistre que ; l'annonce « MISSION TERMINÉE » arrive après le résultat de la commande), réacteurs **à niveau** (ils relisent l'état, perdre un événement ne fausse rien), file **bornée** à 32 et 256 livraisons par `flush`, état **jamais sauvegardé**.

Ce que l'ownership change :

```rust
pub struct Ctx<'a> { content: &'a Content, world: &'a mut World, bus: &'a mut Bus, out: &'a mut Vec<Output>, reacting: bool }
type Reactor = fn(&mut Ctx<'_>, &Event);
const REACTORS: &[Reactor] = &[contacts::on_event, quests::on_event];   // ordre fixe, documenté

fn flush_with(ctx: &mut Ctx<'_>, reactors: &[Reactor]) {
    while budget > 0 {
        let Some(event) = ctx.bus.queue.pop_front() else { break };    // événement possédé, sorti de la file AVANT les réacteurs
        for r in reactors { r(ctx, &event); }                          // un réacteur peut émettre : il ne touche que la file
    }
}
```

- **Pas de registre d'abonnés, pas de `GameState*` aliasé, pas de `Rc<RefCell>`** : des emprunts disjoints (`&mut World`, `&mut Bus`, `&mut Vec<Output>`) dans un `Ctx` construit par appel.
- **La récursion est impossible par construction** : un événement est une valeur sortie de la file avant l'appel des réacteurs ; ce qu'un réacteur émet est livré par la même boucle (`reactors_that_emit_are_delivered_by_the_same_loop` : la quête terminée émet `Reputation`, livré dans le même `flush`, `[E]`).
- **La file n'est pas dans `World`** : « jamais sauvegardé » est un fait de typage.
- **Canal de sortie par phase** : `ctx.reacting` fait écrire `Notice` plutôt que `Line` aux fonctions de règle (`rules::line`) ; les annonces suivent donc le résultat sans que chaque règle le sache.

Testé `[E]` : livraison différée et ordonnée, file bornée (32 + 5 émis → 5 perdus et comptés), **réacteur fou coupé** à exactement 256 livraisons avec file vidée et jeu intact (`a_runaway_reactor_is_cut_off_by_the_delivery_budget`, rendu possible par `flush_with`), idempotence de `quests::refresh` (propriété), l'ordre des réacteurs (un mutant qui l'inverse est attrapé par les deux transcripts : « NOUVEAU CONTACT » doit précéder « Objectif accompli »).

### 4.4 Règles à point d'entrée unique

Tout paiement passe par `World::claim(RewardId) -> bool` (vrai la première fois) : un site rapporte une fois (`site-<id>`), le n-ième scan paie son XP du budget (`scan-<n>`), une quête paie une fois (`quest-<id>`). `quests::complete` pose le statut `Completed` **avant** de payer et refuse une quête non active : deux gardes indépendantes, ce qui rend équivalents deux des mutants (§ 8). L'achat refuse dans un ordre fixe (inconnu, niveau, plafond, crédits) et ne débite jamais sur un refus.

---

## 5. Contenu en données (point 4)

### 5.1 Comparaison

| | Tables `const` Rust + clés typées | RON | **TOML** |
|---|---|---|---|
| Qui l'édite | un développeur | développeur à l'aise | un rédacteur (format universel, éditeurs, `taplo`) `[H]` |
| Texte long, commentaires | chaînes Rust | oui (`r#"…"#`) `[E]` | oui (`"""…"""`) `[E]` |
| Enums et `Option` | natifs | natifs (`Compromise("x")`) | `kind = "compromise"` (étiquette interne, `deny_unknown_fields` fonctionne dans les variantes, `[E]`) |
| Erreur de schéma | erreur du compilateur | `1:48-1:61: Unexpected field named …` : une position, pas la ligne `[E]` | `line 5, column 1`, ligne source et soulignement, nom du champ et champs attendus `[E]` |
| Arbre non typé pour migrer | sans objet | `ron::Value` existe `[E]`, outillage plus pauvre `[H]` | `toml::Table` complet `[E]` |
| `u64` au-delà de `i64::MAX` | natif | oui `[E]` | écrit, lu en typé, **échoue dans un arbre non typé** (§ 9.4) `[E]` |
| Crates dans l'arbre `[E]` | 0 | 14 | **7** |
| Validation « tout texte existe » | chaque clé = une variante d'enum : ≈ 1 300 variantes `[H]`, conflits de fusion | tests | tests |
| Cycles de prérequis, impasses | `const fn` acrobatique ou test | tests | tests |
| Coût d'une retouche de texte | recompilation | rebuild | rebuild : **1,5 s** en dev (1 s pour une retouche de code) `[E]` |

**Recommandation : TOML pour la structure, les textes, les réglages et les sauvegardes** (un seul format, une seule paire de crates `serde` + `toml`). C'est aussi le choix de la bible narrative (`narrative-bible.md` § 6.3, `data/world/` + `data/text/<langue>/`). Un rédacteur ne touche jamais au Rust ; le compilateur reste garant de ce qui est *code* (commandes, `Msg`, enums de schéma). Écartés : RON (2 fois plus de dépendances, messages d'erreur sans extrait, aucune raison de réclamer à un rédacteur une syntaxe Rust) et les tables `const` à clés typées (le C en a une, `strings.def` : 436 entrées ; à 1 300 chaînes par langue l'enum de clés devient un goulot de fusion et ne protège pas des fautes de rédaction, que seule la validation attrape).

### 5.2 Schéma et embarquement

Structure : tableaux `[[site]]`, `[[quest]]`, `[[contact]]`, `[[item]]` (l'ordre du fichier est l'ordre d'affichage : `BTreeMap` perdrait l'ordre, la feature `preserve_order` de `toml` est inutile) ; `#[serde(deny_unknown_fields)]` partout, `tag = "kind"` pour les objectifs, conditions, effets. Extrait réel :

```toml
[[quest]]
id = "m04"
code = "NEXUS_DATA_BREACH"
chapter = 3
giver = "phoenix"
level_required = 3
prereq = ["m03"]
reward = { xp = 80, credits = 200, reputation = 20 }
[[quest.objective]]
kind = "compromise"
site = "nexus-mainframe"
[[quest.objective]]
kind = "reputation"
min = 50
```

Les textes sont sous les mêmes identifiants (`[quest.m04] title, desc, obj = [...]`) dans `data/text/fr/` et `data/text/en/`, aplatis en clés pointées (`quest.m04.obj.1`). Un texte numérique reçoit son nombre **de la donnée** (`{n}` = `min`) : il ne peut pas contredire la donnée (la leçon de `test_blurbs_match_the_code` du C).

**Embarquement par `build.rs`** (30 lignes, aucune dépendance) : parcourt `data/**/*.toml`, génère `EMBEDDED: &[(chemin, include_str!(…))]`, `cargo:rerun-if-changed=data`. Même comportement en debug et en release, et un fichier oublié est impossible (le répertoire est parcouru) ; le moteur ne lit **aucun fichier** : `Content::load(&Sources)` prend des chaînes, la CLI peut lui donner un `--data-dir` de développement. `rust-embed 8.13.0` (lecture disque en debug, embarquement en release : deux comportements) et `include_dir 0.7.4` (non publié depuis juin 2024, d'après la bible narrative) sont écartés `[H]`.

Coûts mesurés `[E]` : `Content::embedded()` 450 µs et `validate()` 107 µs sur le contenu du spike (release) ; **7,6 ms** pour ≈ 1 500 clés × 2 langues synthétiques (release, 40 ms en debug). Parser à chaque démarrage et valider rapidement au démarrage (échec explicite plutôt que partie corrompue) est donc négligeable ; la validation complète reste un test.

### 5.3 Validation

`validate(&Content) -> Vec<ContentError>` rend toutes les erreurs. Chaque ligne est un test **négatif** qui casse une copie des données (`tests/content.rs`) `[E]` :

| Contrôle | Erreur | Cassé par |
|---|---|---|
| schéma : champ inconnu, mauvais type, variante inconnue, modèle de texte mal formé | `LoadError` avec fichier, ligne, colonne (l'erreur dans une variante étiquetée pointe l'en-tête de la table, pas la ligne du champ ; pour un modèle de texte : fichier et clé, sans ligne) | `min_tir`, `sitee`, `"lots"`, `{name` |
| doublons d'id (site, quête, contact, objet, sujet) | `Duplicate` | un `id` recopié |
| références inconnues (donneur, prérequis, site, contact, objet, quête de condition, **nom de commande d'un objectif**) | `UnknownRef` | `giver = "nobody"`, `name = "scna"` |
| prérequis de quêtes acycliques, relais de sites acycliques | `Cycle { chemin }` | `m01` exige `m02` |
| **joueur optimiste** : point fixe sur le contenu seul (scans payés, tous les systèmes atteignables, contacts rencontrés, achats au plus bas prix, quêtes dont les objectifs tiennent), avec et sans le tutoriel | `Unreachable` (une quête, un site ou un contact), `DeadEnd { tier, xp }` | courbe d'XP `[0,15,60,14000]`, réputation de R4Z0R à 1 000 |
| clés de texte : toute clé requise existe dans **chaque** langue ; aucune clé orpheline | `MissingText`, `OrphanText` | ligne retirée, `[site.localhots]` |
| marqueurs : mêmes noms que ceux déclarés par le code (`Msg::SPECS`) ou par la structure ; un singulier pluriel peut omettre le nombre | `Placeholders` | `{gold}`, un `{n}` oublié |
| budgets en **largeur d'affichage** (`unicode-width`) : titre ≤ 28, description ≤ 110, objectif ≤ 64, question ≤ 48… (`narrative-bible.md` § 6.2) | `Budget` | un titre de 54 colonnes |
| caractères interdits : contrôles, espace insécable, `…`, émoji | `ForbiddenChar` | `U+1F600`, `U+2026` |

Le point fixe du « joueur optimiste » généralise `test_no_experience_dead_end` du C. Le scénario « tutoriel passé » (récompense non versée) est l'impasse que `ROADMAP.md` documente déjà (passer le tutoriel coûte 100 ¢ et 10 de réputation) : j'ai **prévenu** celle de mes données en donnant 10 de réputation à `m02`, et le contrôle l'attrape dès qu'on relève l'exigence de R4Z0R (test `detects_an_unreachable_contact_and_its_consequences`). Il a en revanche attrapé une erreur de **mon** validateur dès le premier passage (marqueurs comparés comme liste ordonnée au lieu d'ensemble). Limites `[H]` : il ignore les crédits des achats hors boutique, les bornes de stock et l'ordre des commandes ; le vrai garde-fou final est le robot de `neon-sim` qui joue le **vrai** moteur (§ 8). Les familles encore à écrire (courriers, fragments, cinématiques, fins, décisions : tests T8 à T15 de la bible) suivent le même patron : la structure déclare, `required_texts` en dérive les clés, `validate` croise.

### 5.4 Évolution du contenu et des sauvegardes

À la lecture, `World::fill_missing` ajoute (verrouillé) ce que le contenu a gagné depuis la sauvegarde ; un identifiant **inconnu** du contenu rejette la sauvegarde (`SaveError::Invalid`). Supprimer ou renommer un contenu publié exige donc une migration : c'est voulu (§ 7.2).

---

## 6. i18n (point 5)

**Recommandation : catalogues TOML maison, rendus au moment de l'affichage.**

| | Maison (TOML) | `fluent-bundle 0.16.0` / `fluent 0.17.0` | `rust-i18n 4.2.4` |
|---|---|---|---|
| Syntaxe pour le rédacteur | TOML + `{nom}` | FTL (termes, attributs, sélecteurs) | YAML/JSON/TOML + `t!()` |
| Pluriels | `one`/`other` + règle par langue (0 est singulier en FR) `[E]` | complets (CLDR) | selon backend |
| Vérifier « mêmes marqueurs que le code » | oui : le code déclare ses marqueurs (`Msg::SPECS`) | à écrire sur l'AST FTL `[H]` | non (clés textuelles) `[H]` |
| État global | aucun | aucun | locale globale `[H]` |
| Poids | 0 dépendance en plus (`toml` déjà là) | plusieurs crates `[H]` | macros + chargeur `[H]` |
| À choisir si | FR/EN, 2 formes de pluriel | langues à pluriels riches (russe, arabe…) | non |

Comparaisons Fluent et `rust-i18n` : versions vues par `cargo info`, contenu connu par la documentation, **non compilées ici**.

Mécanique `[E]` :

- **Typé côté code** : `messages! { Hacked = "msg.hacked" { site: String, credits: u32 }; … }` génère l'enum `Msg`, `key()`, `args()` et `Msg::SPECS` (clé + noms de marqueurs). 50 variantes dans le spike. Le code ne peut ni se tromper de clé ni oublier un argument.
- **Dérivé côté données** : `required_texts(content)` liste toutes les clés (`quest.<id>.title/desc/obj.<i>`, `contact.<id>.topic.<t>.q/.a`, `help.<cmd>`, `site.<id>.desc`, `item.<id>.name/.desc`) avec leurs marqueurs autorisés et leur budget. Ajouter une quête ajoute ses exigences sans toucher au test.
- **Parité** : mêmes clés dans chaque langue (`both_languages_cover_exactly_the_required_keys`), mêmes **ensembles** de marqueurs entre eux **et** avec le code, mêmes formes plurielles (la forme `one` peut omettre le nombre), aucune clé orpheline, aucune chaîne vide, aucun caractère interdit.
- **Pluriels** : `Lang::plural(n)` ; français : 0 et 1 singuliers ; anglais : 1 seul `[E]`. Seules `one`/`other` sont modélisées (la catégorie CLDR « many » du français concerne ≥ 10⁶) `[H]`.
- **Au rendu, pas dans le moteur** : le même flux de `Output` s'affiche en FR et en EN (les deux transcripts sont issus du même scénario). Une clé manquante s'affiche `[[clé]]` au lieu de paniquer, et la propriété `no_text_is_missing_in_any_language` joue des parties aléatoires et vérifie, **sur la structure** de chaque texte et dans les deux langues, que la clé existe et que chaque marqueur a son argument `[E]` ; elle a attrapé une faute de frappe de clé injectée dans le code (`contact.echo7.nam`), réduite à un cas minimal. Une première version qui cherchait `[[` dans la chaîne rendue a produit un faux positif : le jeu renvoie au joueur ce qu'il a tapé (`Commande inconnue : ?}`) ; elle a été remplacée par ce contrôle structurel.
- Les noms propres sont des clés (`contact.<id>.name`), identiques dans les deux langues, vérifiées comme les autres.

---

## 7. Sauvegarde (point 6)

### 7.1 Format : TOML

| | TOML | RON | JSON (`serde_json 1.0.151`) |
|---|---|---|---|
| Lisible et diffable dans un rapport de bug, un snapshot `insta` | oui | oui | oui |
| Commentaires | oui | oui | non |
| Arbre non typé pour les migrations | `toml::Table` | `ron::Value` (plus pauvre `[H]`) | `serde_json::Value` |
| Enums à données | étiquette interne (`kind = "conversation"`) ; `Pair = [1, 2]` ; variante unité = chaîne `[E]` | natif | `{"Menu": {…}}` |
| `u64` | contournement (hex, ci-dessous) | natif | natif |
| Arbre de dépendances | 7 | 14 | 5 |

TOML : lisible, un seul format avec le contenu et les réglages, et les migrations s'écrivent sur `toml::Table`. Le sérialiseur de `toml 1.1.6` replace les valeurs simples avant les tables sans erreur `ValueAfterTable` `[E]`. JSON resterait le format des **rapports** de `neon-sim` (lus par des outils).

### 7.2 Schéma, versions, migrations

```toml
version = 2
[meta]   name, tier, credits       # ce qu'affiche la liste des emplacements, lisible sans moteur
[world]  player, sites, quests, contacts, inventory, rewards, commands_run, scans_done, turn, earned_total, rng
[[flows]] kind = "conversation" contact = "r4z0r"      # la pile interactive : la reprise se fait là où l'on s'est arrêté
[end]    ok = true                                      # dernière table
```

- **Règles** : un champ optionnel absent prend sa valeur par défaut (`#[serde(default)]`, changement additif, **pas de nouvelle version**) ; une clé inconnue est ignorée ; un changement **structurel** monte `SAVE_VERSION` et ajoute une migration ; une version supérieure à celle du jeu donne `TooNew`, jamais un chargement approximatif ; version absente, nulle, négative ou non entière : erreur.
- **Migrations sur l'arbre non typé** : `MIGRATIONS[n]: fn(&mut toml::Table) -> Result<(), String>`. `v1_to_v2` (spike) déplace tout sous `[world]`, renomme `rep` en `reputation`, convertit `items = [..]` en `inventory` à compteurs, ajoute `[meta]` et `[end]`. **Une fixture figée par version publiée** (`tests/fixtures/save_v1.toml`, écrite à la main dans le spike ; dans le vrai projet : une vraie sauvegarde recopiée et jamais modifiée) est chargée par un test qui vérifie le résultat champ par champ, puis un aller-retour.
- **Marqueur de fin** : `[end]` est la dernière table ; `end` est un champ **obligatoire** du schéma, donc une sauvegarde tronquée ne peut pas se charger avec des valeurs par défaut. Testé de façon exhaustive : **chaque préfixe** d'une sauvegarde (coupes à toutes les frontières de caractère, > 1 000) est rejeté ; seul le retour à la ligne final peut manquer `[E]`. Le mutant « on ne vérifie plus `end.ok` » survit : équivalent, le schéma exige déjà `[end]` ; la vérification ne protège que d'un `ok = false` fait à la main.
- **État du RNG en hexadécimal** (16 chiffres, `#[serde(with = "hex_u64")]`) : les entiers TOML sont signés sur 64 bits ; `toml 1.1.6` **écrit** un `u64` plus grand que `i64::MAX` tel quel et le relit en typé, mais la lecture dans un `toml::Table` (celle des migrations) échoue avec « u64 value was too large » `[E]`.

### 7.3 Chargement transactionnel et validation

`Engine::from_toml(content, text) -> Result<Engine, SaveError>` **construit un nouvel engin** ; l'appelant ne remplace le sien qu'en cas de succès (`a_failed_load_leaves_the_running_game_untouched`). C'est le « transactionnel » du C obtenu par la signature, sans code de restauration. Après décodage, `validate_world` rejette (`SaveError::Invalid`) ou assainit :

crédits, XP et réputation hors bornes ; site, contact, objet, quête **inconnus** ; objet acheté plus de `max_buys` fois ; plus de drapeaux d'objectif que d'objectifs ; quête démarrée avant ses prérequis ; récompense inconnue (`site-narnia`) ; nom de commande inconnu ; pile de flux trop profonde (> 4) ; flux visant un contact verrouillé ; **nom du joueur assaini** (contrôles, ESC, C1 retirés, 20 caractères, jamais vide : `clean_name`) pour qu'un fichier édité ne puisse pas injecter de séquence d'échappement. Testé `[E]` : 11 cas hostiles dans `hostile_values_are_rejected_or_sanitized`, plus une propriété qui mute une sauvegarde valide à un endroit aléatoire (jamais de panique ; si elle charge, elle se réécrit et se recharge à l'identique).

### 7.4 Écriture atomique

```rust
let mut tmp = tempfile::Builder::new().prefix(".neon-save-").suffix(".tmp").tempfile_in(dir)?; // même dossier = même système de fichiers
tmp.write_all(data)?;  tmp.as_file().sync_all()?;     // données durables avant le renommage
before_rename()?;                                      // crochet de test : injecte la panne
persist(tmp, path)?;                                   // rename(2) / MoveFileExW
sync_dir(dir);                                         // rend le renommage durable (Unix)
```

Testé `[E]` : remplacement correct, dossiers créés, aucun fichier temporaire laissé ; **panne injectée avant le renommage : l'ancien fichier est intact** et le temporaire disparaît ; un lecteur concurrent qui lit 200 fois pendant 50 réécritures de 200-300 Ko ne voit jamais un contenu mélangé.

Sémantique Windows, **lue dans le code** de `tempfile 3.27.0` (`src/file/imp/windows.rs`) : `persist` appelle `MoveFileExW` avec `MOVEFILE_REPLACE_EXISTING` (pas `MOVEFILE_WRITE_THROUGH`), après avoir ôté l'attribut temporaire. Conséquences : le remplacement est atomique, les **données** sont durables (le `sync_all` du temporaire est un `FlushFileBuffers` dans la bibliothèque standard `[H]`), la durabilité du **renommage** n'est pas garantie en cas de coupure, et un destinataire ouvert par un autre processus (antivirus, indexeur) donne `PermissionDenied` : le code réessaie 6 fois avec attente croissante sous `cfg(windows)`. **Cette boucle n'est pas testée** (`cargo check` Windows seulement). `atomic-write-file 0.3.1` n'apporte rien sous Windows : `imp/generic.rs` y fait un simple `fs::rename`, avec un `TODO` pour `MOVEFILE_WRITE_THROUGH`, et il tire 9 crates (`nix`, `rand 0.10`, `chacha20`…).

### 7.5 Emplacements, sauvegarde de secours, quarantaine

```
<données>/saves/slot-1.toml          sauvegarde courante (9 emplacements)
<données>/saves/slot-1.toml.bak      précédente sauvegarde VALIDE, tournée avant chaque écriture
<données>/saves/slot-1.toml.corrupt  fichier illisible, mis de côté au lieu d'être écrasé
<données>/settings.toml
```

- `Slots::save` ne tourne la courante en `.bak` que si `save::peek` la juge valide : une sauvegarde abîmée n'écrase jamais la bonne copie.
- `Slots::load` essaie le `.bak` si la courante est illisible et le dit (« la copie précédente a été chargée »). Sans copie valide : erreur explicite qui propose `--new`, **sans rien écraser** ; avec `--new`, le fichier abîmé devient `.corrupt`.
- Une version trop récente est refusée et le fichier reste intact ; plus de 1 Mio : refus avant lecture en mémoire ; `list()` lit l'en-tête `[meta]` seul et survit aux fichiers cassés.
- Taille : 1 295 octets, 85 lignes après un début de partie ; chargement 30 µs, sérialisation 12 µs `[E]`. Une campagne complète fera quelques dizaines de Kio `[H]` (le C plafonnait à 256 Kio).

### 7.6 Répertoires et réglages

**`directories 6.0.0`** plutôt qu'un code manuel `[C]` (sources lues) : Linux `$XDG_DATA_HOME` **absolu seulement** puis `~/.local/share` (testé par sous-processus : un `XDG_DATA_HOME` relatif est ignoré, `[E]`) ; macOS `~/Library/Application Support/<projet>` ; Windows `%APPDATA%\<projet>\data` (Roaming, avec le suffixe `\data`). `etcetera 0.11.0` (MSRV 1.87) et `dirs 7.0.0` : non évalués. Sauvegardes dans `data_dir` (comme le C avec `%APPDATA%`) ; un dossier local non itinérant (`data_local_dir`) serait à envisager si la synchronisation Roaming gênait `[H]`.

**Priorité des réglages, par réglage** `[E]` (tests unitaires et de bout en bout par le vrai binaire) :

| Réglage | Du plus fort au plus faible |
|---|---|
| langue | `--lang` > `settings.toml` > `$LANG` (deux premières lettres) > anglais |
| couleur | `--color` / `--no-color` > `$NO_COLOR` (force « éteint ») > `settings.toml` > « la sortie est un terminal » |

Deux règles de construction :

- **Environnement hermétique.** `Env` (langue, `NO_COLOR`, terminal, dossier de données par défaut) est capturé **une fois** (`Env::from_process`) et passé par valeur ; `Env::default()` n'a aucun dossier de données. `std::env::set_var` est `unsafe` en édition 2024 et le workspace interdit `unsafe` : les tests d'environnement lancent le **vrai binaire** avec `--print-paths` et un environnement contrôlé. `--no-save` joue sans fichier.
- Un `settings.toml` abîmé donne les valeurs par défaut ; il n'est écrit que s'il n'existe pas (jamais réécrit en silence).

### 7.7 Quand sauvegarder

Après chaque commande exécutée (sauf partie terminée) et à `save` explicite (qui répond « sauvegardée » ou l'échec) ; un échec d'écriture est signalé **une fois** puis ignoré, la partie continue. Les flux étant sérialisables, une sauvegarde en plein menu est valide et se reprend là (testé au niveau du moteur) ; en revanche le spike n'**autosauvegarde pas** pendant un menu : l'écriture suit les commandes exécutées. Choix à confirmer (D4). Une partie terminée n'est pas sauvegardée : « terminée » est transitoire (une partie rechargée n'est jamais « finie », propriété corrigée en conséquence). `[H]` : un `fsync` par commande est négligeable sur SSD ; sur disque lent, regrouper à l'inactivité.

---

## 8. Pyramide de tests (point 7)

| Couche | Outil | Contenu dans le spike (nombre `[E]`) |
|---|---|---|
| Unitaires | `#[test]` dans les modules | ids, RNG (7 suites de référence C), gabarits et pluriels, `Msg`, nom, bus (4) : **15** |
| Contenu | tests d'intégration + copies cassées | contrôles de § 5.3 (positifs et négatifs), coût de démarrage à l'échelle de la campagne : **16** |
| Flux et commandes | tests d'intégration | menus imbriqués, refus, annonces après le résultat, tutoriel indépendant de l'ordre, prologue, fin d'entrée, `help`/`complete`, anti-farm, déterminisme, double complétion : **18** (+ 2 `#[ignore]` : débit et clonage) |
| Propriétés | `proptest 1.11.0` | 10, § ci-dessous : **10** |
| Sauvegarde | tests d'intégration | aller-retour exact, reprise en menu, migration v1, version trop récente, **tous** les préfixes, clés inconnues et absentes, valeurs hostiles, chargement transactionnel : **9** |
| Instantanés | `insta 1.49.0` | transcripts FR et EN du frontend plain ; panneau latéral TUI sur `TestBackend` (feature `tui`) |
| Bout en bout CLI | `Cursor` en mémoire, vrai binaire | reprise entre sessions, sauvegarde de secours, quarantaine, version trop récente, octets hostiles, fin d'entrée à chaque profondeur, environnement hermétique, priorités : **12 + 3 + 6 + 5** |
| Fuzz des chargeurs | `proptest` (fait) ; `cargo-fuzz` (proposé) | propriété `loaders_never_panic` ; `cargo-fuzz` exige nightly et n'est pas installé ici : **non exécuté** `[H]` |
| Simulation | `neon-sim` (à écrire) | robots, métriques, invariants I17-I21 de l'audit `[H]` |

**Propriétés** (`tests/props.rs`), toutes sur des parties **pilotées par l'état** : 17/22 d'entrées valides pour l'invite courante (reconstruites par `complete` et `prompt`), le reste d'entrées hostiles, commandes verrouillées ou `Choice`/`Cancel` parasites. Elles vérifient : les gains ne dépassent jamais le plafond que définit le contenu (anti-farm, I1) et tous les compteurs restent bornés ; aller-retour de sauvegarde depuis tout état atteignable (I3) ; déterminisme (I5) ; invite cohérente, menu toujours quittable, fin définitive ; idempotence de `refresh` (I8) ; chargeurs sans panique sur du texte quelconque et sur des sauvegardes mutées (I4) ; nom assaini (I13) ; gabarits sans panique ; aucun texte manquant dans aucune langue.

**Garde-fou contre le test vide** `[E]` : sur 300 parties générées, 188 atteignent le niveau 2, 175 débloquent R4Z0R, 23 achètent, 219 terminent une quête, 237 passent par un menu, 37 par la boutique, 39 essaient une commande verrouillée. Avec des **lignes purement aléatoires**, seules 8 % atteignaient le niveau 2 et **1 partie sur 300** achetait quelque chose : les propriétés écrites avec ce générateur ne disaient presque rien sur la boutique. D'où le générateur piloté par l'état et le test `generated_games_are_not_vacuous`, qui échoue si l'une de ces portes est franchie moins de 10 fois. Un run long passe : `PROPTEST_CASES=20000` en release, 9 propriétés, 180 000 cas, 41 s, aucun échec `[E]` (un premier run long a trouvé le faux positif décrit au § 6, pas un défaut du moteur). Les graines de régression vont dans `tests/props.regressions` (`WithSource("regressions")` : le défaut `SourceParallel` avertit pour les tests d'intégration).

**Mutations** `[E]` (script jetable : une modification de texte, la suite entière, restauration) : 13 mutants.

| Mutant | Résultat |
|---|---|
| `hack` repaie un site compromis | attrapé (3 tests dont la propriété de plafond) |
| XP des scans non borné | attrapé (2) |
| l'aide liste les commandes verrouillées | attrapé |
| la complétion propose les commandes verrouillées | attrapé |
| cycle de prérequis non détecté | attrapé |
| chargement sans contrôle de `max_buys` | attrapé |
| sérialisation du RNG qui perd l'état | attrapé (3) |
| quête terminable deux fois | **survivait** → test `completing_a_quest_twice_pays_once` ; attrapé |
| ordre des réacteurs inversé | **survivait au moteur seul** ; attrapé par les transcripts de la CLI |
| seuil de rejet du RNG supprimé | **survivait** (plages étroites) → plages larges contre le C ; attrapé |
| quête terminable deux fois **et** sans `claim` (les deux gardes retirées) | attrapé |
| `end.ok` non vérifié | équivalent (le schéma exige `[end]`) |
| récompense de quête sans `claim` | équivalent (le statut `Completed` est une seconde garde) |

Les transcripts `insta` ne sont donc pas décoratifs : ils protègent l'**ordre des annonces**, que rien d'autre ne teste. `cargo-mutants` pourrait automatiser ce passage `[H]` (non installé).

**Correspondance avec les 26 suites C** `[C]` : `test_economy`, `test_alert` → propriété de plafond et règles ; `test_quests` (« pas d'impasse ») → `optimistic_reach` ; `test_events` → tests du bus ; `test_commands`, `test_complete` → registre et complétion ; `test_save`, `test_kv`, `test_storage` → sauvegarde et stockage ; `test_i18n` → parité ; `test_rng` → valeurs de référence du C ; `test_settings`, `test_config` → priorités ; `test_tutorial` → indépendance de l'ordre ; `test_contacts`, `test_shop`, `test_progression`, `test_world` → règles et contenu ; `test_menu`, `test_intro`, `test_io`, `test_lineedit`, `test_parse` → flux et frontends ; `test_hud`, `test_term`, `test_shop_view` → frontends (voir le document TUI).

**Recette d'intégration continue** `[H]`, chaque commande déjà exécutée avec succès sauf mention : `cargo fmt --check` ; `cargo clippy --workspace --all-targets -- -D warnings` (avec et sans `tui`) ; `cargo test --workspace` ; `cargo +1.88.0 test --workspace` ; `cargo check --target x86_64-pc-windows-msvc` et `aarch64-apple-darwin` (fait en `check`) puis **exécution réelle** sur Windows et macOS (non faite) ; nuit : `PROPTEST_CASES=100000 cargo test --release` ; `INSTA_UPDATE=no` ; `cargo-fuzz` en nightly (non fait).

---

## 9. Le spike (point 8)

### 9.1 Contenu

Un moteur et un binaire réels, petits : 10 commandes (`scan`, `hack`, `exploit`, `contact`, `contacts`, `quests`, `status`, `help`, `save`, `quit`), 5 systèmes en réseau avec relais, 4 quêtes (tutoriel compris), 3 contacts avec sujets conditionnels, 3 objets, un prologue (`Continue`, nom, confirmation, tutoriel ou non), une conversation qui ouvre une boutique (menus imbriqués), le bus différé avec deux réacteurs, le contenu TOML embarqué et validé, FR/EN avec pluriels, sauvegarde v2 avec migration depuis v1, écriture atomique, 9 emplacements, réglages, frontend plain, panneau TUI headless.

Tailles `[E]` (`wc -l`) : moteur 3 721 lignes (dont les tests unitaires des modules), tests du moteur 1 319, CLI 732, tests CLI 365, données 457. Le moteur du spike est une **infrastructure** (validation 511, texte 416, commandes 354, contenu 309, flux 307, sauvegarde 294) : le travail de règles du jeu s'y ajoutera (l'audit estime 9,5 à 15 k lignes de moteur au total `[H]`).

### 9.2 Versions exactes

`rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, `clippy 0.1.97`, `rustfmt 1.9.0-stable`, et `rustc 1.88.0` pour la MSRV. Résolues le 7 octobre 2026 par `cargo` (`Cargo.lock`, 228 paquets) `[E]` :

| Crate | Version | Rôle |
|---|---|---|
| `serde` | 1.0.229 | dérivations |
| `toml` | 1.1.6+spec-1.1.0 | contenu, textes, sauvegardes, réglages |
| `thiserror` | 2.0.21 | erreurs typées du moteur |
| `unicode-width` | 0.2.2 | budgets de largeur d'affichage |
| `anyhow` | 1.0.104 | bord du binaire seulement |
| `clap` | 4.6.7 | options (`derive`, `env`) |
| `directories` | 6.0.0 | dossier de données |
| `tempfile` | 3.27.0 | écriture atomique |
| `ratatui` | 0.30.2 (tire `crossterm 0.29.0`) | TUI (feature) ; MSRV 1.88 |
| `proptest` | 1.11.0 | propriétés (dev) |
| `insta` | 1.49.0 | instantanés (dev) |
| `ron` 0.12.2, `serde_json` 1.0.151 | | sonde de formats seulement |

Vues par `cargo info` sans être compilées : `fluent-bundle 0.16.0`, `fluent 0.17.0`, `rust-i18n 4.2.4`, `rand 0.10.3`, `rand_pcg 0.10.2`, `rust-embed 8.13.0`, `atomic-write-file 0.3.1` (source lue), `etcetera 0.11.0`, `dirs 7.0.0`. Doublons dans l'arbre (`thiserror` 1 et 2, `syn` 1, 2 et 3, `getrandom` 0.3 et 0.4, `rand` 0.8 et 0.9, `hashbrown` ×2) : des dépendances transitives de `ratatui` et `proptest`, pas du code du projet.

### 9.3 Résultats `[E]`

| Vérification | Résultat |
|---|---|
| `cargo test --workspace --features neon-cli/tui` | **94 tests, 0 échec**, 6 s une fois compilé (2 `#[ignore]` : débit, clonage) |
| même commande avec `cargo +1.88.0` | 94 tests, 0 échec (MSRV réelle) |
| `cargo clippy --workspace --all-targets --features neon-cli/tui -- -D warnings` | 0 avertissement |
| `cargo fmt --all -- --check` | propre |
| `cargo check --workspace --all-targets --target …` | `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `aarch64-apple-darwin` : compile (sans exécution) |
| build release de `neon-hack`, cible vierge, 4 cœurs | 14 s ; binaire 2,97 Mo non réduit ; session scriptée complète en 2 ms |
| rebuild de développement après retouche d'un fichier de texte / de code | 1,5 s / 1 s |
| débit moteur (release) | 1,14 à 1,39 M commandes/s |
| `Engine::clone` | 447 ns |
| démarrage : charger + valider le contenu du spike / parser ≈ 1 500 clés × 2 | 450 µs + 107 µs / 7,6 ms (release) |
| sauvegarde : taille / sérialisation / chargement | 1 295 o / 12 µs / 30 µs |
| propriétés, run long | 20 000 cas × 9 propriétés, 41 s, 0 échec |
| mutations | § 8 |

### 9.4 Frictions et pièges

1. **`toml` et les `u64` géants** : écrits, relus en typé, **refusés** dans un `toml::Table` (« u64 value was too large »). Contournement : état du RNG en hexadécimal, avec un test d'aller-retour en arbre non typé.
2. **Erreurs dans une variante étiquetée** : `deny_unknown_fields` fonctionne, mais l'erreur désigne l'en-tête `[[quest.objective]]` (ligne 26, colonne 1) et pas la ligne du champ fautif.
3. **`std::env::set_var` est `unsafe` en édition 2024** : impossible d'écrire un test d'environnement sous `unsafe_code = "forbid"` ; d'où `Env` par valeur et des tests par sous-processus.
4. **Un test de ma main a écrit dans le vrai dossier de données** : les transcripts appelaient `app::run` avec la configuration par défaut, qui résolvait `~/.local/share/neon-hack` ; le second test rechargeait la sauvegarde du premier (visible dans le transcript français, qui ne commençait pas par le prologue). Corrigé **par construction** : `Env::default()` n'a plus de dossier de données ; `--no-save` ajouté. Nettoyage : j'ai supprimé uniquement ce que j'avais créé (`saves/` et `settings.toml`) ; le `savegame.sav` du jeu C, antérieur, n'a pas été touché, et le dossier ne contient plus que lui en fin de session `[E]`. Par précaution, j'ai ensuite lancé les tests avec un `HOME` jetable.
5. **`clippy`** : `allow-unwrap-in-tests` ne couvre pas les fonctions d'aide des tests d'intégration (160 avertissements) ; `build.rs` déclenche les interdits du moteur (`fs::write`) ; les interdits s'appliquent aussi aux tests (un `Instant` de mesure de débit) : `allow` ciblés et commentés.
6. **Générateur d'entrées** : 1/300 parties achète quelque chose avec des lignes aléatoires ; un générateur piloté par l'état et un garde-fou de couverture corrigent. Le générateur piloté par l'état a ensuite révélé que `quit` valide terminait 79 % des parties : exclu des entrées valides.
7. **`proptest`** : `FileFailurePersistence::SourceParallel` avertit pour les tests d'intégration ; fixer `cases` explicitement ignore `PROPTEST_CASES` (relire la variable à la main).
8. **`insta`** : `cargo-insta` n'est pas installé ; `INSTA_UPDATE=always` crée les `.snap`, qu'il faut relire (`tests/snapshots/`).
9. **Une propriété fausse, pas un bogue** : « sauvegarder puis recharger redonne la même invite » est faux pour une partie terminée ; `ended` est transitoire par conception.
10. **Mon validateur s'est trompé en premier** : comparaison des marqueurs en liste ordonnée au lieu d'ensemble ; corrigé (les marqueurs sont triés).
11. **`let` chaînés** (`if let … && …`) : stables en 1.88, édition 2024 ; ils fixent la MSRV autant que `ratatui`.
12. **Résolveur sensible à la MSRV** : avec `rust-version = "1.88"`, `cargo` a choisi `uuid 1.26.1` et signalé que la 1.27.0 exige 1.89 `[E]` ; sans cette clé il aurait pris la plus récente `[H]`. Ne pas retirer `rust-version`.

### 9.5 Non vérifié

- Comportement **à l'exécution** sous Windows et macOS (seulement `cargo check`) ; la boucle de réessai `PermissionDenied` sous Windows ; le dossier de données Windows/macOS (lu dans le code de `directories`, non exécuté).
- `cargo-fuzz`, `cargo-mutants`, `cargo-deny`, `cargo-insta` (non installés ; le passage de mutations était un script jetable).
- Les comparaisons avec Fluent, `rust-i18n`, `rust-embed`, `etcetera` (documentation et versions, pas de compilation).
- Tout ce qui n'est pas dans le spike : intrusions et solveur, courriers, fragments, cinématiques, `undo`, commande d'indice, modes d'accessibilité (`@sr`, `--ascii`), `Role`/`Importance`/`Screen` du document TUI, newtypes saturants, `legal_inputs()` publique, `neon-sim`.
- Les estimations de taille à l'échelle de la campagne (clone 3 à 5 fois plus cher, sauvegardes de quelques dizaines de Kio, 1 300 chaînes par langue) sont des extrapolations.
- Débits mesurés sur un bac à sable à 4 cœurs, état de la taille du spike ; contenu de 4 quêtes, 5 systèmes, 3 contacts, 3 objets : les contrôles `O(n²)` et les recherches linéaires par id (`Content::site`) sont négligeables à cette taille ; indexer par `BTreeMap` si le contenu dépasse la centaine d'éléments `[H]`.

### 9.6 Reproduire

```sh
cd <scratchpad>/spike-engine
HOME=$(mktemp -d) cargo test --workspace --features neon-cli/tui          # 94 tests (HOME jetable : jamais les vraies sauvegardes)
cargo clippy --workspace --all-targets --features neon-cli/tui -- -D warnings
cargo +1.88.0 test --workspace --features neon-cli/tui                    # MSRV
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
PROPTEST_CASES=20000 cargo test --release -p neon-engine --test props     # run long
cargo test --release -p neon-engine --test engine_flows -- --ignored --nocapture   # débit, clonage
printf '\nNeo\ny\n1\nscan\nstatus\nquit\n' | cargo run -q -p neon-cli -- --no-save --seed 1 --lang en
```

---

## 10. Décisions à valider

| # | Question | Options | Recommandation |
|---|---|---|---|
| D1 | Style de l'API du moteur | machine à états + pile de flux ; `trait Ui` à lecture bloquante ; coroutine / `async` | **Machine à états** : seule à permettre `undo`, solveur, sauvegarde en plein menu et boucle de la TUI, pour un coût de texte passe-partout par flux (§ 3.2) |
| D2 | Format des données | TOML partout (contenu, textes, réglages, sauvegardes) ; RON ; tables `const` Rust ; JSON | **TOML partout**, embarqué par `build.rs`, validé en tests et au démarrage ; JSON réservé aux rapports de simulation (§ 5.1, 7.1) |
| D3 | Contrat de déterminisme | moteur pur (ni horloge, ni E/S, ni `HashMap`, imposé par `clippy`), PCG32 maison identique au C ; ou `Clock` et `rand` injectés | **Pur**, graine en paramètre, temps = `turn`, horodatage côté CLI. Conséquence : la durée de jeu n'est pas dans le moteur (§ 3.4) |
| D4 | Politique de sauvegarde et de compatibilité | 9 emplacements, sauvegarde auto à chaque commande, `.bak` d'un niveau, quarantaine `.corrupt` ; promesse de migration à partir de la 1.0 (fixture figée par version) | **Oui**, migrations obligatoires dès la 1.0, aucune garantie avant. À croiser avec le mode « sans retour » de l'audit (D3) : désactiver `.bak` et sauvegarde en cas de défaite (§ 7.5) |
| D5 | Chaîne d'outils | MSRV 1.88 (plancher de `ratatui 0.30.2`, vérifiée), édition 2024, TUI activée par défaut ; ou plain par défaut | **MSRV 1.88**, `tui` par défaut dans les binaires publiés, construction plain-only aussi en CI ; monter la MSRV seulement sur exigence d'une dépendance (§ 2.2, 2.3) |
| D6 | i18n | catalogues TOML maison (`{nom}`, `one`/`other`) ; Fluent ; `rust-i18n` | **Maison** pour FR/EN. **Si une troisième langue à pluriels riches est envisagée, choisir Fluent maintenant** (§ 6) |
| D7 | Identifiants et niveaux de validation du contenu | ids textuels stables + validation complète en tests + rapide au démarrage (échec explicite) ; ou index d'enum | **Ids textuels**, démarrage en échec explicite sur contenu invalide ; un renommage de contenu publié exige une migration (§ 4.1, 5.3, 5.4) |
| D8 | Place de la mécanique de piratage dans l'architecture | une intrusion = `Flow::Run` derrière une interface (`AutoResolve` d'abord, tactique ensuite) ; ou intégrée aux commandes | **`Flow::Run` derrière une interface**, conforme à la recommandation A de l'audit ; la campagne complète se joue d'abord en `AutoResolve`. À valider avec D1 de l'audit (§ 3.3) |

---

## Annexe : arborescence du spike

```
spike-engine/
  Cargo.toml  rustfmt.toml  Cargo.lock
  data/world/{progression,sites,quests,contacts,items}.toml
  data/text/{fr,en}/{ui,content}.toml
  crates/neon-engine/
    build.rs  clippy.toml
    src/{ids,rng,text,msg,content,state,events,rules,quests,contacts,commands,flows,output,engine,validate,save,lib}.rs
    tests/{common/mod,content,engine_flows,props,save}.rs  tests/fixtures/save_v1.toml
  crates/neon-cli/
    clippy.toml
    src/{main,lib,app,plain,settings,storage,tui}.rs
    tests/{transcript,storage,paths}.rs  tests/snapshots/transcript__transcript_{en,fr}.snap
spike-formats/   sonde RON / TOML / JSON (hors du workspace)
spike-mut/       copie jetable des mutations (run_mutants.py)
```
