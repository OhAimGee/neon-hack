# Livraison : migration vers `legacy-c/`, squelette Rust, CI et releases

> **Mise à jour du 7 octobre 2026 : table rase du C** (`DECISIONS.md` P3 et R-1). La migration vers `legacy-c/` (§ 1), le job `legacy-c.yml` (§ 3) et le critère de suppression du C (D5) sont **abandonnés** : le C est supprimé dès R0, son historique reste dans git. Le reste (squelette Rust § 2, CI § 3 hors `legacy-c`, releases § 4, flux § 5) reste valable ; les lints et les licences sont à corriger comme indiqué dans `CROSS-CHECK.md` (16, 15).

**Légende des preuves** — `[E]` exécuté ici (commande lancée, résultat lu ; rustc/cargo 1.97.0, Linux, 2026-10-07) ; `[C]` lu dans un fichier du dépôt, dans le source d'un outil ou d'une action sans l'exécuter ; `[D]` connaissance de la documentation d'une plateforme, **non vérifiée ici** ; `[H]` hypothèse ou choix de conception. Ce qui n'a pas pu être vérifié sans GitHub est regroupé au § 7.

Tous les essais ont eu lieu dans des copies jetables sous `scratchpad/` ; le dépôt n'a reçu que ce fichier. Les brouillons livrables sont dans `scratchpad/ci-drafts/` (liste au § 8).

## Résumé

1. **Déplacer le C dans `legacy-c/` ne demande aucune modification du Makefile, des tests ni des scripts** : tous leurs chemins sont relatifs au dossier courant `[C]`. Après le déplacement, `make`, `make test` (26 suites, 36 453 vérifications unitaires, 105 e2e, 75 pty) et `make asan` donnent **exactement les mêmes comptes qu'avant**, avec gcc, clang et ASan, 0 échec `[E]`. Il faut en revanche corriger : `.gitignore` (l'exécutable est ancré à la racine), la CI, les deux README, deux liens de documentation, et sortir un `.pyc` suivi par erreur. Un script de 105 lignes fait le tout en deux commits (109 renommages purs, puis les corrections) ; il a été exécuté sur une copie et l'historique suit (`git log --follow`, `git blame`) `[E]`.
2. **Squelette R0** : espace de travail à 3 crates (`neon-engine`, `neon-cli`, `neon-sim`), édition 2024, résolveur 3, MSRV **1.88** (imposée par `ratatui 0.30.2`), lints partagés. `cargo fmt --check`, `clippy -D warnings`, `cargo test` (7 tests), `cargo doc -D warnings`, `cargo deny check` (4 contrôles, base RustSec téléchargée en direct) passent ; le tout compile aussi avec `rustc 1.88.0` et pour 6 cibles de livraison `[E]`.
3. **Deux garde-fous d'architecture réellement vérifiés par mutation** : Clippy interdit dans `neon-engine` l'horloge, l'environnement, le disque, la console et `HashMap` (fichier `clippy.toml` propre au crate) ; `cargo-deny` interdit à `neon-engine` de dépendre de `ratatui` ou `clap` (`wrappers`). Chaque règle a été violée volontairement et la CI aurait échoué `[E]`.
4. **CI** : `ci.yml` (fmt, clippy et tests sur 3 systèmes, MSRV, cargo-deny, documentation, un contrôle unique `CI OK` à exiger), `legacy-c.yml` (filtré par chemins, Ubuntu 24.04 fixé), `release.yml`, Dependabot (cargo, actions, `rust-toolchain`). `actionlint 1.7.12` + `shellcheck 0.11.0` : 0 erreur ; `zizmor 1.30.1` (audit de sécurité) : 0 résultat moyen ou grave ; schémas JSON GitHub : valides `[E]`. Toutes les actions sont épinglées par SHA de commit, résolu sur les dépôts officiels.
5. **Release : matrice écrite à la main, pas `cargo-dist`.** `cargo-dist 0.32.0` a été installé, configuré et exécuté : son workflow fait 296 lignes (le nôtre 186 + 111 lignes de scripts testables en local), installe `dist` par `curl | sh`, n'épingle aucune action, et `zizmor` y relève **19 résultats graves** `[E]`. Sa valeur (installeurs, Homebrew, MSI) ne sert pas avant la v1.0.
6. **Version tirée du tag** : `build.rs` (11 scénarios exécutés) + contrôle `check-release.sh` (tag ≃ `Cargo.toml` ≃ `CHANGELOG`, commit sur `main`) `[E]`.
7. **Six décisions** à valider au § 6.

---

## 0. Constats de départ

| Constat | Preuve |
|---|---|
| 115 fichiers suivis : `src/` 65, `tests/` 33 (dont un `.pyc`), `docs/` 7, `Makefile`, 3 `demo_*.sh`, 2 README, `LICENSE`, `.gitignore`, `.gitattributes`, `ci.yml` | `git ls-files` `[E]` |
| CI actuelle : 1 workflow de 38 lignes, 3 jobs (gcc, clang, ASan), `ubuntu-latest`, `actions/checkout@v4` ; derniers jobs : 46 s, 48 s, 53 s | `ci.yml` `[C]` ; API Actions, run le plus récent `[E]` |
| Le workflow remonte les échecs de test **en annotation** « car les journaux sont illisibles sans droits admin » (commit `eaed18e`) : à conserver pour le Rust | `git log` `[E]` |
| Dépôt **public** (`OhAimGee/neon-hack`, licence CC0), branche par défaut `main`, 2 branches distantes (`main`, `refonte/v1`), aucune release, tag distant `legacy-v2.087` → `af69f9b` (absent en local) | API GitHub et `git ls-remote` `[E]` |
| Historique de fusion : `Merge pull request #1` (PR `refonte/v1` → `main`), puis fusion locale ; messages en français de la forme `portée: résumé` (`quêtes:`, `CI:`, `docs:`, `tests:`) | `git log` `[E]` |
| La protection de branche et les réglages de fusion **ne sont pas lisibles** (API : 403) | `[E]` |
| `.pyc` suivi : `tests/e2e/__pycache__/pty_hud.cpython-312.pyc` (déjà noté ROADMAP phase 6) | `git ls-files` `[E]` |
| Outils disponibles : rustup 1.28.2, Docker (client), Python 3.13 ; `cargo-deny`, `cargo-about`, `cargo-dist`, `actionlint`, `zizmor`, `shellcheck` installés dans `scratchpad/tools/` pour l'occasion | `[E]` |

---

## 1. Migration du C vers `legacy-c/`

### 1.1 Ce qui bouge, ce qui reste

| Reste à la racine | Va dans `legacy-c/` |
|---|---|
| `.gitattributes`, `.gitignore`, `LICENSE`, `README.md`, `README.en.md`, `docs/ROADMAP.md` (à réécrire pour le Rust), `docs/design/` (nouveaux documents), `.github/` | `Makefile`, `src/` (65), `tests/` (33), `demo_*.sh` (3), `docs/ARCHITECTURE.md` (décrit le C), `docs/legacy/` (5 rapports d'époque) |

`.github/workflows/ci.yml` est **renommé** `legacy-c.yml` (même historique). Au total **109 renommages** dans le premier commit ; 100 % de similarité : `git diff -M --numstat` ne montre que le `.pyc` (binaire) `[E]`.

### 1.2 Ce qui ne change pas, et pourquoi

Rien à éditer dans ces fichiers : ils sont résolus depuis le dossier courant, qui devient `legacy-c/` quand on y lance `make` `[C]` puis `[E]`.

| Fichier | Lignes qui citent un chemin | Pourquoi ça tient |
|---|---|---|
| `Makefile` | 13-14 (`build/default`, `neon_hack`), 25-28 (`src/main.c`, `src/game/*.c`, `tests/unit/test_*.c`), 72 et 74 (`tests/e2e/run.sh`, `tests/e2e/pty_hud.py`), 79 (cible `asan`) | tous relatifs ; `VERSION` utilise `git rev-parse --short HEAD`, valable depuis un sous-dossier (`neon_hack 0.1.0-dev+c638da8` obtenu) |
| `tests/e2e/run.sh` | l. 10 `BIN=${NEON_HACK_BIN:-./neon_hack}` | aucun `dirname`/`$0` |
| `tests/e2e/pty_hud.py` | l. 27 `os.environ.get("NEON_HACK_BIN", "./neon_hack")` | aucun `__file__` |
| `tests/unit/*.c` | `#include "../../src/…"` | `tests/` et `src/` se déplacent ensemble |
| `demo_*.sh` | `./neon_hack` | à lancer depuis `legacy-c/` |

### 1.3 Éditions exactes (commit 2)

| Fichier | Édition |
|---|---|
| `.gitignore` | l. 5-6 `/neon_hack`, `/neon_hack.exe` → `/legacy-c/neon_hack`, `/legacy-c/neon_hack.exe` (`build/`, `*.o` ne sont pas ancrés : suffisent). Ajouter `__pycache__/` et `*.pyc`. **Sans cela, `legacy-c/neon_hack` apparaît comme fichier non suivi après un build** `[E]` |
| index git | `git rm --cached legacy-c/tests/e2e/__pycache__/pty_hud.cpython-312.pyc` |
| `.github/workflows/legacy-c.yml` | `name: Legacy C` ; `on.push.paths` et `on.pull_request.paths` = `legacy-c/**` et le fichier lui-même ; `push.branches: [main]` seulement ; `defaults.run.working-directory: legacy-c` ; `runs-on: ubuntu-24.04` (au lieu de `latest` : le C est en `-Werror`, un gcc plus récent ne doit pas le casser seul) ; `timeout-minutes`, `persist-credentials: false`, `checkout` épinglé par SHA ; le reste (matrice gcc/clang, annotation d'erreur) est identique |
| `README.md` | 6 remplacements : note « le jeu en C vit dans `legacy-c/`, `cd legacy-c` » sous `## Compiler et lancer` ; lien `docs/ARCHITECTURE.md` → `legacy-c/docs/ARCHITECTURE.md` ; 4 lignes du bloc « Structure » préfixées `legacy-c/` ; ligne `docs/` |
| `README.en.md` | les 6 mêmes, en anglais |
| `legacy-c/docs/ARCHITECTURE.md` | lien `(ROADMAP.md)` → `(../../docs/ROADMAP.md)` ; note « chemins relatifs à `legacy-c/` » |
| `docs/ROADMAP.md` | note de tête : plan du C, gelé ; chemins relatifs à `legacy-c/` (ce fichier sera réécrit pour le Rust) |

Le script `migration/migrate-to-legacy-c.sh` applique tout cela ; chaque remplacement est **asserté une seule occurrence** (il échoue bruyamment si le README a dérivé). Il refuse de tourner sur `main` ou sur un arbre sale.

**Pas de `Makefile` de transfert à la racine** : `make` à la racine échouera avec « No targets », message clair ; un second Makefile serait un fichier de plus à supprimer. Le README dit `cd legacy-c`.

### 1.4 Vérification (copie jetable `legacy-move-test3`) `[E]`

| Étape | Résultat |
|---|---|
| Avant (copie intacte) : `make test` | 38,5 s ; 26 suites / 36 453 vérifications ; 105 e2e + 75 pty (180 lignes `ok`) ; 0 échec |
| Après, **sans aucune édition** (renommages seuls) : `make -C legacy-c test` | 37,3 s ; mêmes comptes |
| Après le script complet, depuis `legacy-c/` : `make CC=gcc test` | mêmes comptes, 0 échec |
| idem `make CC=clang test` / `make asan` | mêmes comptes, 0 échec (ASan : 49,7 s) |
| `git status` après build | propre (seul `docs/design/` non suivi, hors sujet) |
| Historique | `git log --follow legacy-c/src/game/shop.c` : 7 entrées (6 d'origine + le déplacement) ; `git blame` rend les auteurs et commits d'origine ; `ci.yml` → `legacy-c.yml` garde ses 2 commits d'origine (`857ef27`, `eaed18e`) en plus des 2 nouveaux |
| `actionlint` / `zizmor` sur l'arbre migré | 0 erreur / aucun résultat |

Deux commits plutôt qu'un : le premier se relit d'un coup d'œil (que des renommages), le second porte le jugement. Le premier commit n'est pas compilable par la CI (le workflow lance encore `make` à la racine) : il ne faut donc pas faire de `git bisect` à travers lui (`git bisect skip`). Fusion par **commit de fusion** (§ 5.1) : la détection de renommage par contenu fonctionne aussi après un squash, mais deux commits gardent la relecture facile.

### 1.5 Pièges relevés

- **Noms des contrôles de CI** : les jobs `build + tests (gcc)`, `build + tests (clang)`, `tests under ASan + UBSan` gardent leur nom, mais le workflow s'appelle maintenant « Legacy C » (et non « CI »). Si la protection de `main` exige ces noms (illisible ici), les PR resteraient bloquées : **à vérifier dans Settings → Branches avant de fusionner** `[D]`.
- **Un contrôle filtré par chemins ne doit pas être exigé** : sur une PR qui ne touche pas `legacy-c/`, le workflow ne démarre pas et le contrôle exigé reste « en attente » à jamais `[D]`. D'où le contrôle unique `CI OK` du Rust (§ 3), qui, lui, tourne toujours.
- `docs/ROADMAP.md` et `legacy-c/docs/ARCHITECTURE.md` citent de nombreux chemins `src/…` : la note de tête suffit, on ne réécrit pas l'historique du C.
- Le README promet toujours le tag `legacy-v2.087` : il **existe** sur le dépôt distant, il manque seulement en local (`git fetch --tags`).

### 1.6 Vie et suppression de `legacy-c/`

- **Critères de suppression** (à valider, décision D5) : (1) un test e2e « campagne complète » en Rust joue les 10 quêtes jusqu'à l'épilogue en mode `--plain`, et la TUI est jouable ; (2) tout ce que le README du C déclare fonctionnel est couvert ou explicitement abandonné dans `docs/design/game-systems-audit.md` ; (3) le README décrit l'installation du binaire Rust.
- **PR « retrait de legacy-c »** : taguer d'abord le dernier commit qui le contient (`git tag -a legacy-c-final -m "Dernier état du jeu en C"`, poussé), puis `git rm -r legacy-c .github/workflows/legacy-c.yml`, retirer de `.gitignore` les lignes `/legacy-c/…` et Python, retirer la note des README. Simulé sur la copie intégrée : après suppression, `fmt`, `clippy -D warnings`, 5 suites de tests, `cargo deny`, `actionlint` passent ; `git show legacy-c-final:legacy-c/Makefile` retrouve le C `[E]`. Le Rust ne dépend en rien de `legacy-c/`.
- Pendant sa vie, `legacy-c/` est **gelé** : seules les corrections qui font échouer sa CI y entrent.

---

## 2. Squelette Rust (R0)

### 2.1 Fichiers (28, plus `legacy-c.yml` qui appartient à la PR de migration)

```
Cargo.toml  Cargo.lock  rust-toolchain.toml  rustfmt.toml  clippy.toml  deny.toml
about.toml  about.hbs  CHANGELOG.md  .gitignore (modifié)  .cargo/config.toml
crates/neon-engine/{Cargo.toml, clippy.toml, src/lib.rs}
crates/neon-cli/{Cargo.toml, build.rs, src/main.rs, tests/cli.rs}
crates/neon-sim/{Cargo.toml, src/main.rs}
.github/workflows/{ci.yml, release.yml}   .github/actions/setup-rust/action.yml
.github/scripts/{run-annotated.sh, check-release.sh, package.sh}
.github/dependabot.yml   .github/pull_request_template.md
```

Les trois crates sont des **talons** (≈ 100 lignes de moteur, tests compris) : ils existent pour que tout le pipeline s'exerce sur du vrai code (sérialisation `serde`, erreur `thiserror`, propriétés `proptest`, binaire `clap`, test du binaire compilé). Le contenu réel vient des autres chantiers.

### 2.2 Choix, avec la preuve de chacun

| Choix | Raison | Preuve |
|---|---|---|
| `resolver = "3"` + `rust-version = "1.88"` | Cargo choisit des versions de dépendances **compatibles avec la MSRV**, même avec un rustc plus récent | `Locking 60 packages to latest Rust 1.88 compatible versions` ; `uuid 1.27 (requires Rust 1.89.0)` écartée au profit de 1.26.1 `[E]` |
| MSRV **1.88** | `ratatui 0.30.2` l'exige (doc TUI § 7.1) | `cargo +1.88.0 check --workspace --all-targets --locked` passe **y compris l'arbre complet avec les dev-dependencies** ; `cargo +1.88.0 test` passe sur le squelette `[E]` |
| `rust-toolchain.toml` : `channel = "1.97.0"` exact | `clippy -D warnings` casserait au hasard à chaque sortie de Rust (nouveaux lints) ; la mise à jour est une PR dédiée | Dependabot gère l'écosystème `rust-toolchain` pour les versions exactes (`dependabot-core`, `common/lib/dependabot/config/file.rb:90` et `rust_toolchain/…/channel_type.rb`) `[C]` ; le schéma JSON de `dependabot.yml` l'accepte `[E]` |
| `rustup toolchain install` **sans argument** dans la CI | lit `rust-toolchain.toml` (version, profil, composants) | exécuté avec rustup 1.28.2 : installe 1.88.0, idempotent ; note : ce rustup a même installé la toolchain tout seul au premier `cargo`, mais la CI ne s'y fie pas `[E]` |
| `[workspace.dependencies]` : toutes les versions à un seul endroit | une montée de version = une ligne | 13 crates, versions lues sur crates.io le 2026-10-07 (tableau § 2.3) `[E]` |
| `[workspace.lints]` : `unsafe_code = forbid`, `clippy::pedantic`, `unwrap_used`/`expect_used`/`print_stdout`/`dbg_macro`/`todo` en `warn` (donc erreurs sous `-D warnings`), conversions numériques `warn` | état sauvegardé : pas de panique cachée, pas de troncature silencieuse de crédits ; `std::env::set_var` est `unsafe` en édition 2024 | mutations ci-dessous `[E]` |
| `allow-expect-in-tests` ne couvre **pas** les fonctions d'aide d'un fichier `tests/*.rs` | constaté : `expect` dans `tests/cli.rs` rejeté | règle : chaque `tests/*.rs` commence par `#![allow(clippy::expect_used, clippy::unwrap_used)]` `[E]` |
| `print_stdout` interdit sauf dans le module `plain` | un `println!` dans la TUI corrompt l'écran | `main.rs` : `mod plain { #![allow(clippy::print_stdout)] … }` ; un `println!` ailleurs est rejeté `[E]` |
| `crates/neon-engine/clippy.toml` (propre au crate) | **Clippy lit un seul fichier, celui du crate, sans fusion avec la racine** : on y répète les `allow-*-in-tests` | voir mutations `[E]` |
| `overflow-checks = true` en release | sans cela, `0 - 1` sur `u32` donne 4 294 967 295 crédits (l'économie du C a connu des farms) | `[H]`, réglage présent ; les tests (debug) vérifient déjà les débordements |
| `panic` laissé à `unwind` | le `Drop` du garde de terminal doit s'exécuter (doc TUI § 4.5 : avec `abort` seul le hook agit) | `[C]` doc TUI |
| `.cargo/config.toml` : alias `lint`, `play`, `sim` ; `+crt-static` sous Windows MSVC | `cargo lint` = exactement le job Clippy ; pas besoin du « Visual C++ Redistributable » | alias non exécutés sous Windows ; `cargo check --target x86_64-pc-windows-msvc` passe avec le drapeau `[E]` ; l'effet à l'édition de liens est `[D]` |
| Pas de `default-members` | sinon un `cargo test` oublié de `--workspace` ne testerait que `neon-cli` | choix `[H]` ; `cargo play -- --plain` lance le jeu |
| `Cargo.lock` suivi | le dépôt livre un binaire ; `--locked` en CI | `[H]` ; après une montée de version, `cargo update -w` ne touche que les 3 crates du dépôt et `--locked` repasse `[E]` |
| `publish = false`, licence `CC0-1.0` | aucun accès à crates.io par erreur ; la licence du dépôt | `[C]` `LICENSE` |

**Mutations (les règles ont des dents)** `[E]` — dans une copie, `neon-engine` reçoit volontairement `Instant::now()`, `thread::sleep`, `HashMap`, `env::var`, `println!`, `unwrap()` : Clippy produit **8 diagnostics** pour ces 6 violations (`HashMap` est signalé à chaque mention). Dans `neon-cli`, `Instant::now()` n'est **pas** rejeté (la config est bien propre au moteur), `println!` hors du module `plain` l'est. `unsafe { … }` est rejeté partout (`forbid`). Enfin, `cargo deny` : `neon-engine` qui dépend de `ratatui` et `clap` → **2 erreurs `banned`** ; chaque dépendance interdite précise ses `wrappers` (seuls parents directs autorisés) : `anstream` est tirée par `clap_builder`, `signal-hook 0.3.18` par `signal-hook-mio` (les deux wrappers ont dû être ajoutés après un premier échec) `[E]`.

### 2.3 Dépendances partagées (déclarées, non toutes utilisées par R0)

| Crate | Version | MSRV | Licence | Rôle |
|---|---|---|---|---|
| `serde` (derive) | 1.0.229 | 1.56 | MIT/Apache-2.0 | sérialisation (moteur) |
| `thiserror` | 2.0.21 | 1.77 | MIT/Apache-2.0 | erreurs (moteur) |
| `clap` (derive) | 4.6.7 | 1.85 | MIT/Apache-2.0 | options (neon-cli) |
| `ratatui` | 0.30.2 | 1.88 | MIT | TUI (neon-cli) ; `ratatui::crossterm`, pas de `crossterm` direct |
| `unicode-width` / `unicode-segmentation` | 0.2.2 / 1.13.3 | 1.66 / 1.85 | MIT/Apache-2.0 | largeur et graphèmes |
| `anstream` | 1.0.0 | 1.66 | MIT/Apache-2.0 | sortie couleur du plain |
| `signal-hook` | 0.4.5 | 1.66 | MIT/Apache-2.0 | Unix : restauration du terminal |
| `proptest` | 1.11.0 | 1.85 | MIT/Apache-2.0 | propriétés (dev) |
| `insta` | 1.49.0 | 1.66 | Apache-2.0 | snapshots (dev) |
| `portable-pty` / `vt100` | 0.9.0 / 0.16.2 | — / 1.70 | MIT | tests pty (dev, `cfg(unix)`) |
| `tempfile` | 3.27.0 | 1.63 | MIT/Apache-2.0 | dossiers temporaires de test (dev) |

Le format texte de la sauvegarde (`toml 1.1.6` ou `ron 0.12.2`, évalués par le chantier « formats ») est **laissé vide** dans `Cargo.toml` : la liste de licences autorisées couvre déjà les deux.

Arbre complet (`ratatui` + tout ce qui précède + `toml` + `ron`, copie `full-tree`) : 97 crates externes sur l'hôte Linux (92 pour `x86_64-pc-windows-msvc`), 135 avec les dev-dependencies, 204 toutes plateformes confondues (`cargo metadata`) ; compile ensemble, MSRV 1.88 comprise ; licences rencontrées : MIT, Apache-2.0, Unicode-3.0, Zlib, CC0-1.0 (le projet), et BSL-1.0 en alternative d'Apache-2.0 pour `ryu` `[E]`. D'où la liste autorisée de **5 licences** dans `deny.toml` : toute autre (BSD, ISC, MPL, GPL…) devra être ajoutée par une PR qui dit pourquoi.

### 2.4 Validation du squelette `[E]`

| Commande | Squelette R0 | Arbre complet |
|---|---|---|
| `cargo fmt --all --check` | ok | — |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 avertissement | — |
| `cargo test --workspace --locked` | 7 tests ok (4 moteur dont 2 `proptest`, 3 binaire) | — |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --locked` | ok | — |
| `cargo +1.88.0 check --workspace --all-targets --locked` | ok | ok |
| `cargo deny check` (advisories, bans, licenses, sources ; base RustSec du jour) | 4 ok | 4 ok |
| `cargo check --workspace --all-targets --target …` | — | ok pour `x86_64`/`aarch64-pc-windows-msvc`, `x86_64`/`aarch64-apple-darwin`, `aarch64-unknown-linux-musl` |
| `cargo build --release --target x86_64-unknown-linux-musl` | binaire statique (`static-pie`, 803 Ko pour le talon), `--version` ok | 45 s à froid |
| `cargo about generate` | `THIRD-PARTY-LICENSES.txt` de 1 681 lignes, 50 textes distincts | — |

**Limites** : les liens et l'exécution sous macOS et Windows n'ont pas eu lieu ici (compilation seule, depuis Linux) ; `cargo-deny` affiche des avertissements `unused-wrapper` tant que `ratatui` & co ne sont pas des dépendances réelles (inoffensif, disparaît).

---

## 3. GitHub Actions

### 3.1 Vue d'ensemble

| Workflow | Déclencheurs | Rôle |
|---|---|---|
| `ci.yml` | PR, push `main`, `workflow_call`, `workflow_dispatch`, lundi 05:17 UTC | contrôle du Rust ; réutilisé par la release |
| `legacy-c.yml` | PR et push `main` **filtrés** sur `legacy-c/**` | garde le C vert tant qu'il existe (§ 1) |
| `release.yml` | tag `vX.Y.Z` ou `vX.Y.Z-…` | binaires, sommes, attestation, release GitHub |
| `dependabot.yml` | hebdomadaire | crates, actions (par SHA), `rust-toolchain.toml` |

`permissions: contents: read` en tête de chaque workflow ; seul le job `publish` reçoit `contents: write`, `id-token: write`, `attestations: write`. Une action locale `setup-rust` centralise le choix du compilateur (toolchain du dépôt, ou version demandée, puis cache) : un seul endroit à changer, et aucune écriture dans `$GITHUB_ENV` (le job MSRV appelle `cargo +1.88.0`).

### 3.2 `ci.yml` : les jobs

| Job | Ce qu'il fait | Pourquoi |
|---|---|---|
| `fmt` | `cargo fmt --all --check` | le formatage est le même partout (LF imposé par `.gitattributes` et `newline_style = "Unix"`) : un système suffit |
| `clippy` × 3 OS | `cargo clippy --workspace --all-targets --locked -- -D warnings` | sur **les trois** systèmes : le code `cfg(windows)` n'est vu que par Clippy sous Windows (ROADMAP : « Windows jamais compilé ») |
| `test` × 3 OS | `cargo test --workspace --locked`, `INSTA_UPDATE=no`, `timeout-minutes: 25` | un test pty suspendu ne bloque pas 6 heures ; pty `cfg(unix)` donc ignoré sous Windows (doc TUI § 4.6) |
| `msrv` | `cargo +1.88.0 check --workspace --all-targets --locked` + garde : la MSRV du workflow égale `rust-version` de `Cargo.toml` | verrouille la promesse du § 2.2 |
| `deny` | `cargo-deny` : bans, licences, sources (action `EmbarkStudios/cargo-deny-action` v2.1.1, qui embarque cargo-deny **0.20.2**, celui validé ici) | frontières d'architecture, licences |
| `advisories` | `cargo-deny` : failles RustSec | **non exigé** : une faille publiée aujourd'hui ne doit pas bloquer une PR qui ne touche pas aux dépendances ; **bloquant** dans la passe hebdomadaire (courriel) |
| `docs` | `cargo doc --no-deps --document-private-items`, `RUSTDOCFLAGS=-D warnings` | liens rompus = erreur |
| `toolchain-drift` | (planifié/manuel) clippy + tests avec le prochain `stable` | prévenir avant la PR Dependabot de la toolchain ; jamais bloquant |
| **`CI OK`** | échoue si un job requis n'est pas `success` | **le seul contrôle à exiger** dans la protection de branche |

`fail-fast: false` partout (un échec sous Windows ne masque pas macOS), `concurrency` annule les anciens runs d'une PR. Douze jobs par PR ; sur un dépôt public les minutes sont gratuites `[D]`. Mesure locale du talon : clippy 6 s, tests 7 s, build release musl 45 s ; **le temps réel sur runner est inconnu**.

### 3.3 Annotations d'échec

`run-annotated.sh "<titre>" <commande…>` lance la commande et, en cas d'échec, émet une annotation `::error` (erreurs rustc/Clippy et leur position, noms des tests en échec, message d'`assert_eq!` avec left/right, diffs rustfmt) et ajoute la fin du journal au résumé du job. C'est le pendant Rust de ce que fait déjà `ci.yml` pour le C. Exercé sur un vrai échec de test, un diff rustfmt, une erreur de compilation et un échec muet ; échappement des `%`, `:` et `,` vérifié `[E]`.

### 3.4 Sécurité et épinglage

Toutes les actions sont épinglées par **SHA de commit**, résolu sur le dépôt officiel de l'action (`git ls-remote`). Attention : quand le tag est *annoté*, `ls-remote` renvoie l'objet tag, pas le commit ; il faut le SHA « pelé » (`^{}`) — deux actions concernées, corrigées `[E]`.

| Action | Version | SHA de commit |
|---|---|---|
| `actions/checkout` | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `Swatinem/rust-cache` | v2.9.2 | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` |
| `EmbarkStudios/cargo-deny-action` | v2.1.1 | `3c6349835b2b7b196a839186cb8b78e02f7b5f25` |
| `actions/upload-artifact` | v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |
| `actions/download-artifact` | v8.0.2 | `9000827ccba6bdab643e8b6fd33ac0654aef8333` |
| `actions/attest-build-provenance` | v4.2.2 | `4d101475d8b20a2381f78447822ac1eab6504dd8` |

Entrées des actions relues dans leur `action.yml` (pas devinées) ; `persist-credentials: false` partout ; le job de release ne lit aucun cache (`cache: "false"` : un binaire publié ne sort pas d'un cache partagé). `zizmor` : 10 résultats, **tous de niveau « bas »** (8 autres sont supprimés par ses règles de bruit) : une suggestion de syntaxe pour les actions locales, volontairement non adoptée car non vérifiable ici.

### 3.5 Ce qui a été validé, et ce qui ne peut pas l'être sans GitHub

Validé `[E]` : YAML chargé par Python ; `actionlint 1.7.12` avec `shellcheck 0.11.0` et `pyflakes` : 0 erreur sur les 3 workflows ; schémas JSON officiels (`vendor.github-workflows`, `vendor.github-actions`, `vendor.dependabot` de `check-jsonschema 0.38.2`) valides ; `zizmor 1.30.1` hors ligne ; les scripts passent `shellcheck` ; chaque commande des jobs a été rejouée à la main (§ 2.4), ainsi que la logique de `CI OK` (4 cas de `jq`), la garde MSRV et le test de fumée du binaire de release.

**Non validable ici** (liste au § 7) : l'exécution réelle sur GitHub, les runners macOS/Windows, l'action Docker de cargo-deny, les audits en ligne de `zizmor` (le jeton injecté est refusé), Dependabot.

---

## 4. Release

### 4.1 La chaîne

`tag vX.Y.Z` → `verify` (forme du tag, version de `Cargo.toml`, section de `CHANGELOG.md`, commit sur `main`) → `ci` (la **même** `ci.yml`, appelée en `workflow_call`) en parallèle de `licenses` (`cargo about`, 2 min 25 s de compilation mesurées) → `build` × 4 → `publish`.

| Cible | Runner | Archive | Test de fumée |
|---|---|---|---|
| `x86_64-unknown-linux-musl` | ubuntu-latest | `.tar.gz` | oui |
| `aarch64-apple-darwin` | macos-latest | `.tar.gz` | oui |
| `x86_64-apple-darwin` | macos-latest (compilation croisée) | `.tar.gz` | non (Rosetta non garanti) |
| `x86_64-pc-windows-msvc` | windows-latest | `.zip` | oui |

Linux est livré **statique (musl)** : aucune dépendance à la version de glibc du joueur ; le binaire se lie sans `musl-gcc` `[E]`. Extensions prévues mais commentées dans la matrice : `aarch64-unknown-linux-musl` sur `ubuntu-24.04-arm` et `aarch64-pc-windows-msvc`, **à n'ajouter qu'après avoir vérifié que le runner existe** (un libellé inexistant laisse le job en file d'attente indéfiniment).

Chaque archive contient : le binaire, `README.md`, `README.en.md`, `LICENSE`, `CHANGELOG.md`, `THIRD-PARTY-LICENSES.txt`. Elle est accompagnée de son `.sha256` ; `publish` les concatène en `SHA256SUMS`, **relit** la somme (`sha256sum -c`), signe une attestation de provenance, crée la release en **brouillon** puis la publie : un échec en route ne laisse qu'un brouillon invisible. Pré-version automatique si le tag contient un `-`. Archives `package.sh` testées en local : tar.gz (binaire `rwx` conservé, extraction et `--version` ok), zip (7z, sinon zip, sinon `python -m zipfile`, chemin Python exercé), somme relue `[E]`.

### 4.2 Version tirée du tag

Source de vérité de SemVer : `[workspace.package].version` de `Cargo.toml` (le tag doit être `v<version>`). `build.rs` calcule la chaîne affichée par `--version`, par priorité : (1) `NEON_HACK_VERSION` (la release l'impose depuis le tag) ; (2) dans un clone git : le nom du tag si HEAD est exactement sur `v*`, sinon `<version>-dev+<sha court>` (comme le Makefile actuel) ; (3) la version de Cargo. Scénarios exécutés `[E]` :

| # | Situation | `neon-hack --version` |
|---|---|---|
| 1 | commit sans tag | `0.1.0-dev+dfa4169` |
| 2 | nouveau commit, sans `clean` | `0.1.0-dev+3675916` (suit HEAD) |
| 3 | HEAD sur le tag `v0.1.0` | `0.1.0` |
| 4 | un commit après le tag | `0.1.0-dev+2fdf2ad` |
| 5 / 6 | `NEON_HACK_VERSION=v9.9.9` / valeur invalide (`a b`) | `9.9.9` / retombe sur git |
| 8 | tag supprimé | `-dev+…` |
| 9 | tag `v0.2.0` (Cargo à 0.1.0) | `0.2.0` — d'où le contrôle de la release |
| 10 | sans dossier `.git` | `0.1.0` |
| 11 | `git worktree` | `0.2.0` (le tag est vu) |

**Limite mesurée** : un build fait là où git est absent garde sa version de repli jusqu'à `cargo clean -p neon-cli` ; Cargo ne peut pas surveiller un `.git` qui n'existe pas (un chemin absent dans `rerun-if-changed` recompilerait le crate à **chaque** build, constaté). Sans effet sur les releases. Un tag annoté a d'abord donné une version périmée pour cette même raison, puis la bonne après `touch build.rs`.

`check-release.sh` (testé : 5 formes de tag invalides, version désaccordée, section absente, commit hors `main` simulé avec un `origin` local, tag introuvable) fait échouer la release avant toute compilation. Il extrait aussi les notes de release du `CHANGELOG.md`.

### 4.3 `cargo-dist` ou matrice écrite à la main

`cargo-dist 0.32.0` (crate publiée le 2026-05-22 ; le dépôt a des tags `v0.33.0` et `v1.0.0-rc.1`, le projet vit) a été compilé (2 min 48 s), configuré (`dist init --yes`, quatre cibles) et exécuté (`dist plan`, `dist build` pour musl) `[E]`.

| Critère | `cargo-dist` | Matrice manuelle (ce dossier) |
|---|---|---|
| Taille | `release.yml` généré de **296 lignes** + `dist-workspace.toml` (13) | `release.yml` 186 lignes + 111 lignes de scripts testables en local |
| Premier résultat | immédiat… mais le workspace en `publish = false` « n'a rien à publier » : il faut `dist = true` dans `[package.metadata.dist]` (lu dans le message d'erreur) | nécessite d'écrire les scripts |
| Archives | `neon-cli-<cible>.tar.xz` (nom du **paquet**), contient le binaire et `CHANGELOG.md` seulement ; `.xz` : 315 Ko (binaire + `CHANGELOG.md`) ; notre `.tar.gz` de 406 Ko contient aussi README, LICENSE et 82 Ko de licences tierces | `neon-hack-<version>-<cible>.tar.gz\|zip` avec README, LICENSE, licences tierces |
| Sécurité (`zizmor`) | **32 résultats : 19 graves** — 16 actions non épinglées (tags), permissions excessives, `curl … \| sh` de l'installeur de dist sans somme, image non épinglée, injections de gabarit | **0 moyen ou grave** |
| Garde tag/version | intégrée (`--tag v0.2.0` contre Cargo 0.1.0 : refusé) | `check-release.sh` (équivalent, plus : `CHANGELOG`, commit sur `main`) |
| Réutiliser la CI, licences tierces, test de fumée, brouillon puis publication | possible par « jobs personnalisés », donc du YAML à écrire de toute façon | natif |
| Installeurs shell/PowerShell/MSI, Homebrew | oui | non |
| Maintenance | workflow régénéré à chaque montée de `dist` (0.x, format qui bouge) | stable, épinglé par Dependabot |

**Recommandation : la matrice manuelle** (décision D2). Revoir `cargo-dist` après la v1.0 si l'on veut un Homebrew tap ou un MSI.

### 4.4 Licences tierces

MIT et Apache-2.0 imposent de redistribuer leur texte avec le binaire. `about.toml` accepte les 5 licences de `deny.toml` (même liste : en changer une impose de changer l'autre) et ne regarde ni les dev-dependencies ni les dépendances de build ; `about.hbs` produit un fichier texte bilingue joint à chaque archive. Exécuté sur l'arbre complet : 1 681 lignes `[E]`.

### 4.5 Limites d'installation (à dire dans le README)

Les binaires ne sont **pas signés** : macOS (Gatekeeper : un binaire téléchargé par navigateur porte l'attribut de quarantaine ; `xattr -d com.apple.quarantine neon-hack` ou téléchargement par `curl`) et Windows (SmartScreen) afficheront un avertissement `[D]`. La notarisation Apple et la signature Windows coûtent un compte payant : hors périmètre ; un Homebrew tap ou `winget` les contourneraient plus tard. L'attestation de provenance (`gh attestation verify <fichier> --repo OhAimGee/neon-hack`) et `SHA256SUMS` prouvent l'origine, pas l'identité d'un éditeur.

### 4.6 Publier une version

1. PR `release/vX.Y.Z` : monter `[workspace.package].version`, `cargo update -w` (ne touche que les 3 crates du dépôt), renommer `## [Non publié]` en `## [X.Y.Z] - date` (et rouvrir un `[Non publié]` vide), mettre à jour la section Installation du README ; **CI OK** vert ; fusion.
2. Avant une version mineure : le **protocole d'accessibilité manuel** (doc TUI, LEC-9 : NVDA + Windows Terminal, Narrator, VoiceOver, Orca, brltty).
3. En local sur `main` à jour : `bash .github/scripts/check-release.sh vX.Y.Z` (répétition à blanc, sans `CHECK_ON_MAIN`).
4. `git tag -a vX.Y.Z -m "Neon Hack X.Y.Z"` puis `git push origin vX.Y.Z`.
5. Suivre le workflow ; vérifier la release : `sha256sum -c SHA256SUMS`, `gh attestation verify`, lancer chaque binaire (`--version` affiche `X.Y.Z`, sans `-dev`) sur les trois systèmes.
6. Échec : voir § 5.4. **Ne jamais déplacer un tag publié** ; un tag dont aucune release n'est sortie peut être supprimé et recréé.

### 4.7 Dependabot ou Renovate

**Dependabot** : natif, rien à installer. Trois écosystèmes, un jour par semaine : `cargo` (groupe « mineur + correctif » en une PR), `github-actions` (`/` et `/.github/actions/*`, groupées), `rust-toolchain`. Délai de carence de 7 jours (`cooldown`, recommandé par `zizmor`). Renovate offre davantage (regroupements, épinglage de digests, cron fin) mais exige l'installation d'une application ou d'un exécuteur ; non justifié pour un mainteneur. Les mises à jour de `ratatui` (0.x : tout changement mineur peut casser) passent par la CI, dont les snapshots `insta` signalent un changement de rendu. `directories` avec un motif et l'écosystème `rust-toolchain` sont acceptés par le schéma JSON `[E]` ; leur effet réel n'est visible qu'après la première exécution (onglet Insights → Dependency graph → Dependabot) `[D]`.

---

## 5. Flux de travail

### 5.1 Branches et PR par phase

- **`main` est toujours verte et livrable.** Tant que `legacy-c/` existe, le README y pointe pour jouer ; le Rust avance dans `crates/` sans rien casser (arbres indépendants).
- **Une PR = un lot livrable** (comme les « lots 3.1 à 3.5 » du plan C), idéalement < 600 lignes de diff hors snapshots et contenu. Pas de branche d'intégration de longue durée : la dérive coûterait plus que la fréquence des PR. L'ancienne pratique (`refonte/v1`, 24 commits, une grosse PR) est abandonnée pour cette raison `[H]`.
- **Noms de branche** : `rust/<lot>-<slug>` (ex. `rust/r1-2-sauvegarde`), `contenu/<slug>` (textes seuls), `docs/…`, `ci/…`, `fix/…` ; Dependabot : `dependabot/…`. Les sessions d'assistant (`claude/…`) suivent le même flux : PR, jamais de push sur `main`.
- **Fusion** : *commit de fusion* (pas de squash), comme la PR n° 1 : les commits d'un lot restent lisibles ; la branche est supprimée après fusion. Le journal reste `portée: résumé`, sans fusions fast-forward.
- **Fin de phase Rn** : une PR « fin de phase Rn » (cases de `docs/ROADMAP.md` cochées, `CHANGELOG.md`, doc d'architecture à jour). Pré-version taguée quand la phase produit un exécutable jouable : `v0.N.0-alpha.1`, … ; `v1.0.0-rc.1` puis `v1.0.0` pour la campagne complète `[H]`.
- Séquence de lancement : **PR R0a** (migration, § 1) fusionnée, puis **PR R0b** (squelette, § 2) ; entre les deux, la CI est celle de `legacy-c.yml`.

### 5.2 Convention de commit (français)

Prolonge l'usage existant (`portée: résumé`, minuscules, sans point final) :

```
<portée>: <résumé à l'infinitif ou au nom, ≤ 72 caractères> [(lot R1.2)]

<corps facultatif : POURQUOI, pas quoi ; 80-100 colonnes>
```

Portées : crate (`engine`, `cli`, `sim`), `tui`, `plain`, `i18n`, `contenu`, `a11y`, `docs`, `ci`, `deps`, `build`, `tests`, `legacy-c`. Une rupture de format de sauvegarde ajoute au corps une ligne `Rupture : …` et monte `SAVE_FORMAT_VERSION`. Les contributions d'un assistant portent le pied `Co-Authored-By:` demandé par l'outil. Les messages de Dependabot restent en anglais (préfixes `deps:`, `ci:`, `build:` configurés).

### 5.3 Définition de « terminé »

**Par lot** (la case à cocher est `.github/pull_request_template.md`) : fmt, clippy, tests verts en local ; **CI OK** vert ; un cas nominal, un cas limite, un cas d'échec, et une propriété `proptest` si une règle a des bornes ; aucune règle d'équilibrage codée en dur ; aucune E/S dans `neon-engine` ; chaque chaîne affichée dans le catalogue FR **et** EN (test de parité) ; l'effet d'une action dit en mots (pas seulement par la couleur) et vérifié en `--plain` et `--no-color` ; tout état nouveau sérialisé avec un test d'aller-retour ; documentation en français et commentaires de code en anglais ; une puce dans `CHANGELOG.md` ; toute dépendance justifiée et `cargo deny check` vert.

**Par phase** : tous les lots fusionnés ; `cargo test --workspace` vert sous les 3 systèmes ; MSRV verte ; cases de la feuille de route cochées ; transcripts de référence à jour ; aucun `#[ignore]` sans ticket.

**Par release** : § 4.6, y compris le protocole d'accessibilité manuel et le contrôle des binaires téléchargés.

### 5.4 Échecs de CI : conduite à tenir

1. **Lire l'annotation** (page de la PR, onglet Checks) : elle contient l'erreur, sa position ou le test en échec ; le journal complet demande d'être connecté. Reproduire : `cargo lint`, `cargo test --workspace --locked`, `cargo +1.88.0 check --workspace --all-targets --locked`, `cargo deny check`. Un échec propre à Windows ou macOS : `cargo check --target x86_64-pc-windows-msvc` ou `aarch64-apple-darwin` depuis Linux détecte les erreurs de compilation (passe aujourd'hui `[E]`) ; l'exécution ne se reproduit que sur le runner.
2. **Classer** : (a) défaut du code → corriger ; (b) test instable (pty, délai) → **un** relancement autorisé (`gh run rerun --failed`) ; le second échec est un bug : ticket « instable : <test> », jamais `#[ignore]` sans ce ticket ; (c) infrastructure (runner, réseau, crates.io) → relancer ; (d) dérive (toolchain, dépendance, faille) → PR dédiée.
3. **`advisories` rouge** : traiter dans la semaine ; mettre à jour la dépendance, sinon une entrée `ignore` **avec raison** dans `deny.toml`. Le contrôle hebdomadaire rouge envoie un courriel.
4. **Jamais de fusion sur rouge.** `main` rouge : on arrête la chaîne ; correctif immédiat sinon `git revert -m 1 <fusion>` dans l'heure.
5. **Release en échec** : le brouillon reste invisible ; corriger, supprimer le brouillon, et refaire un tag si rien n'a été publié, sinon publier un correctif `vX.Y.(Z+1)`.

### 5.5 Réglages GitHub à appliquer (hors dépôt, illisibles ici)

Protection de `main` : PR obligatoire, contrôle **`CI OK`** exigé (et lui seul), pas de poussée forcée ; règle de dépôt interdisant la création de tags `v*` à qui n'est pas le propriétaire (la release part d'un simple `push` de tag) ; jeton `GITHUB_TOKEN` en lecture seule par défaut ; alertes et mises à jour de sécurité Dependabot activées ; suppression automatique des branches fusionnées ; vérifier que la protection actuelle n'exige pas d'anciens noms de contrôles (§ 1.5).

---

## 6. Décisions à valider

| # | Question | Recommandation |
|---|---|---|
| D1 | Compilateur : version **exacte** épinglée (1.97.0, mise à jour par PR Dependabot) ou `stable` flottant ? MSRV 1.88 | Épingler (Clippy ne casse pas au hasard) ; MSRV 1.88 vérifiée en CI |
| D2 | Release : matrice manuelle ou `cargo-dist` ? | Matrice manuelle (§ 4.3) ; réexaminer `cargo-dist` après la v1.0 |
| D3 | Ce que livre la v1.0 : 4 binaires (Linux x86_64 statique, macOS arm64 et x86_64, Windows x86_64), archives + `SHA256SUMS` + attestation, **non signés** ; ni installeur, ni Homebrew, ni winget, ni crates.io | Oui ; la signature et les installeurs viendront après, si la demande existe |
| D4 | Flux : petites PR par lot vers `main`, fusion par commit de fusion, pré-versions `v0.N.0-alpha.K` ; ou une branche d'intégration `rust/main` de longue durée comme `refonte/v1` | Petites PR (§ 5.1) |
| D5 | Fin du C : supprimer `legacy-c/` dès que le plain joue la campagne complète et que la TUI est jouable (e2e « campagne » vert), avec le tag `legacy-c-final`, sans attendre la v1.0 | Oui (§ 1.6) |
| D6 | Appliquer les réglages GitHub du § 5.5 (protection de `main` sur `CI OK`, règle de tags `v*`, jeton en lecture seule, Dependabot) | Oui : ils ne peuvent pas être faits depuis le dépôt |

---

## 7. Vérifié et non vérifié

**Exécuté ici** `[E]` : migration en copie jetable (3 copies, trois compilations : gcc, clang, ASan, comptes identiques avant et après) ; historique (`--follow`, `blame`) ; script de migration ; squelette (fmt, clippy, test, doc, MSRV 1.88 sur l'arbre complet, 6 cibles en `check`, build musl statique) ; mutations Clippy et `cargo-deny` ; `cargo-deny 0.20.2` avec la base RustSec du jour ; `cargo-about 0.9.2` ; `cargo-dist 0.32.0` (init, plan, build local) ; `build.rs` (11 scénarios) ; `check-release.sh`, `package.sh`, `run-annotated.sh` ; YAML (Python), `actionlint`, `shellcheck`, `zizmor` hors ligne, schémas JSON ; tags et SHA des actions par `git ls-remote` ; état du dépôt par l'API publique (visibilité, runs, tags) ; arbre de dépendances et licences.

**Lu sans exécuter** `[C]` : sources de `dependabot-core` (écosystème `rust-toolchain`), `Dockerfile` et `entrypoint.sh` de `cargo-deny-action` (cargo-deny 0.20.2, contournement de rustup 1.28 déjà prévu), `action.yml` de 5 actions.

**Non vérifiable sans GitHub ou sans la plateforme** :
- l'exécution réelle des workflows (permissions effectives, expressions `continue-on-error`/`concurrency`, appel `workflow_call` de `ci.yml` par `release.yml`, durées, cache) ;
- macOS et Windows : `7z` dans le PATH de Git Bash, `sha256sum`/`shasum`, `tee /dev/stderr`, édition de liens avec `+crt-static`, exécution des tests (pty sous macOS) ;
- l'action Docker de cargo-deny (Docker est présent ici, l'image n'a pas été tirée) ;
- les audits en ligne de `zizmor` (SHA « imposteurs », actions vulnérables) : le jeton injecté est refusé ; les SHA ont été résolus sur les dépôts officiels, ce qui couvre l'essentiel du risque ;
- Dependabot (ouverture réelle des PR, `directories` avec motif) ;
- l'existence du runner `ubuntu-24.04-arm` pour ce compte ;
- la protection de branche et les réglages de fusion (API : 403) ;
- Gatekeeper et SmartScreen sur des binaires non signés ;
- `gh attestation verify` et l'attestation de provenance `[D]`.

## 8. Livrables de travail (hors dépôt)

`scratchpad/ci-drafts/skeleton/` : les 29 fichiers (R0 + `legacy-c.yml`) ; `scratchpad/ci-drafts/migration/migrate-to-legacy-c.sh` ; copies d'essai : `scratchpad/legacy-move-test{,2,3}/` (migration), `ci-drafts/r0-integration/` (migration + squelette, puis suppression de `legacy-c`), `ci-drafts/full-tree/` (arbre de dépendances complet), `ci-drafts/lint-mutation/`, `ci-drafts/version-test/`, `ci-drafts/dist-eval/`, `ci-drafts/pkg-test/`.

## 9. Risques

- **Windows et macOS jamais exécutés** : le premier run réel peut révéler des différences de shell ou de chemin. Atténuation : lancer R0b sur une branche et lire les trois systèmes avant tout autre lot ; les scripts sont en Bash sur les trois (`defaults.run.shell: bash`).
- **Tests pty sous macOS** (doc TUI : `[H]`, non exécutés) : si instables, les isoler derrière une variable d'environnement plutôt que les ignorer.
- **Compilateur épinglé** : une PR Dependabot de toolchain peut être rouge (nouveaux lints) ; elle est le lieu prévu pour les corriger.
- **Fichiers non signés** : avertissements Gatekeeper/SmartScreen pour les joueurs (§ 4.5).
- **`CI OK` mal exigée** : si la protection exige des jobs isolés, un job renommé bloque les PR ; n'exiger que `CI OK`.
- **Dérive de la liste de licences** : une dépendance transitive sous une licence nouvelle fera échouer `cargo deny` à la montée de version ; c'est voulu, la correction est une ligne dans `deny.toml` **et** `about.toml`.
