# 🌆 Neon Hack

RPG textuel cyberpunk pour le terminal. Vous êtes un hacker novice à Neo-Tokyo, en 2087, face à la mégacorporation Nexus Corp et à son mystérieux Projet Aurora.

> Jeu généré par IA en 2025 et laissé inachevé, puis refondu une première fois en C. Il est maintenant **réécrit entièrement en Rust, en repartant de zéro**.

**Français** · [English](README.en.md)

## Statut

**Le jeu n'est pas encore jouable** : le dépôt contient le squelette technique en Rust (trois crates, CI, règles de lint) mais pas encore le moteur de jeu. Le cadrage est terminé ; le socle (phase R1) est la prochaine étape.

| Étape | État |
|---|---|
| Cadrage et dossiers de conception (systèmes de jeu, narration, TUI et accessibilité, architecture, CI) | fait : [`docs/design/`](docs/design/README.md) ; les spécifications détaillées (phase P1) et plusieurs décisions de design restent à valider |
| Feuille de route de la v1.0 | écrite : [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Ancien code C et fichiers parasites | supprimés (voir [Ancienne version](#ancienne-version-en-c)) |
| Phase R0 : squelette Cargo, CI sur trois systèmes | fait |
| Phase R1.1 : contrat moteur ↔ frontends, deux interfaces (plain et plein écran), jeu de démonstration | fait |
| Phase R1.2 : textes en TOML embarqués, pluriels, `--ascii` strictement 7 bits, contrôles de parité FR/EN | fait |
| Phases R1.3 à R1.5 : sauvegarde, commandes et réglages, jeu jouet complet | **prochaine étape** |
| Moteur de jeu, TUI, campagne | à venir |

## Jouer en local

> **Le jeu complet n'existe pas encore.** Ce que l'on peut lancer aujourd'hui est la **démonstration du moteur** : un mini-jeu (prologue, commandes, boutique, jauge d'alerte) qui montre les deux interfaces. Il n'y a pas de binaire précompilé avant la v1.0 : on compile le projet soi-même, ce qui prend de quelques dizaines de secondes à quelques minutes la première fois (téléchargement des dépendances compris).

### 1. Installer les outils (une seule fois)

- **Git**, pour télécharger le projet ([git-scm.com](https://git-scm.com)).
- **Rust**, via [rustup](https://rustup.rs) (sous Linux et macOS : `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` ; sous Windows : `rustup-init.exe`). Le fichier `rust-toolchain.toml` du projet choisit la bonne version du compilateur ; si `cargo` signale qu'elle manque, lancez `rustup toolchain install`.
- **Un éditeur de liens**, que Rust utilise pour fabriquer l'exécutable :
  - Windows : « Build Tools pour Visual Studio », charge de travail *Développement Desktop en C++* (`rustup-init.exe` le propose) ;
  - macOS : `xcode-select --install` ;
  - Linux : `gcc` (par exemple `sudo apt install build-essential`).
- **Un terminal moderne en UTF-8** : Windows Terminal (pas l'ancienne console `cmd`), Terminal ou iTerm2 sous macOS, n'importe quel terminal Linux courant. Sous Windows, WSL convient aussi, mais c'est alors un environnement Linux : installez Git, Rust et `gcc` **dans WSL**, en suivant les étapes Linux ci-dessus (pas les Build Tools), puis lancez les commandes dans le terminal WSL.

### 2. Télécharger le projet

```bash
git clone https://github.com/OhAimGee/neon-hack.git
cd neon-hack
```

Sans Git : sur la page GitHub du projet, *Code → Download ZIP*, décompressez, puis ouvrez un terminal dans le dossier obtenu.

### 3. Lancer la démonstration

```bash
cargo run --release -p neon-cli -- --demo
```

La première fois, Cargo télécharge et compile les dépendances ; les lancements suivants sont immédiats. Pour obtenir une commande `neon-hack` utilisable depuis n'importe où :

```bash
cargo install --path crates/neon-cli --locked
neon-hack --demo
```

`rustup` a déjà placé `~/.cargo/bin` dans le `PATH`. Pour mettre à jour : avec un clone, `git pull` puis relancez la commande ci-dessus (avec `--force` pour `cargo install`) ; avec le ZIP, téléchargez-le de nouveau. Pour désinstaller : `cargo uninstall neon-cli`.

### 4. Comment jouer

Dans un vrai terminal, la démonstration s'ouvre en **plein écran** (barre d'état, journal, panneau latéral). Si l'entrée est redirigée, ou avec `--plain`, elle se joue **ligne par ligne**.

- Au début : *Entrée* pour continuer, un pseudo, puis `o` ou `n` pour confirmer.
- Commandes : `help` liste ce qui est possible ; `status`, `scan`, `shop` (la boutique) et `quit`.
- Menus : tapez le **numéro** d'une entrée (dans l'étal, `proxy`, `cloak` et `deck` marchent aussi) ; `0` ou une ligne vide revient en arrière (*Échap* en plein écran).
- Plein écran : *TAB* complète une commande, *Ctrl+D* ou *Ctrl+C* termine la partie, puis une touche ferme l'interface.

| Option | Effet |
|---|---|
| `--plain` | interface ligne par ligne |
| `--screen-reader` | mode lecteur d'écran : interface ligne par ligne, sans symboles à épeler |
| `--ascii` | ASCII 7 bits : symboles, accents et texte tapé translittérés, sans décoration |
| `--lang fr` / `--lang en` | langue des textes (anglais par défaut) |
| `--verbosity brief\|normal\|full` | quantité d'ambiance affichée |
| `--seed 7` | partie reproductible |

L'interface plein écran demande au moins **64×20** caractères, et **100×28** pour afficher le panneau latéral ; en dessous, un message le dit : agrandissez la fenêtre ou utilisez `--plain`.

### En cas de problème

- `cargo: command not found` : fermez puis rouvrez le terminal après avoir installé Rust, ou ajoutez `~/.cargo/bin` au `PATH`.
- `linker 'cc' not found` ou `link.exe not found` : installez l'éditeur de liens (étape 1).
- Cadres, symboles ou accents mal affichés : le terminal n'est probablement pas en UTF-8 ; essayez `--ascii`, qui n'écrit plus que de l'ASCII sur 7 bits (les accents deviennent des lettres simples).
- Une erreur à propos de la version de Rust : lancez `rustup toolchain install` dans le dossier du projet.

## Développer

Prérequis : [rustup](https://rustup.rs) (le fichier `rust-toolchain.toml` épingle la version du compilateur).

```bash
cargo test --workspace      # tests unitaires, de propriété et du binaire
cargo lint                  # clippy, exactement comme la CI
cargo run -p neon-cli --    # lance le binaire (un talon pour l'instant)
```

## Objectifs de la v1.0

1. **Une campagne complète** : début, milieu, fin et épilogue, avec des choix qui comptent.
2. **Une interface TUI plein écran** avec panneau latéral, et un frontend « plain » équivalent (tubes, tests, lecteurs d'écran).
3. **Une accessibilité intégrée dès le premier lot** : `NO_COLOR`, palettes à fort contraste, mode lecteur d'écran, aucune pression de temps.
4. Français et anglais complets ; Linux, macOS et Windows natifs ; binaires publiés.

## Documentation

Les documents sont écrits en français.

- [`docs/ROADMAP.md`](docs/ROADMAP.md) : phases, risques, hors périmètre.
- [`docs/design/DECISIONS.md`](docs/design/DECISIONS.md) : décisions prises, décisions à valider.
- [`docs/design/README.md`](docs/design/README.md) : index des dossiers de conception et règle de préséance.
- [`docs/design/CROSS-CHECK.md`](docs/design/CROSS-CHECK.md) : contradictions entre les dossiers et leur résolution.

## Ancienne version en C

Le jeu en C a été retiré du dépôt, mais son historique est intact :

- dernier état du C : commit `653bc46` (`git checkout 653bc46`, puis `make` ; ses sources se lisent aussi par `git show 653bc46:src/game/world.c`) ;
- version d'origine, avant toute refonte : tag `legacy-v2.087`.

## Licence

[CC0 1.0 Universal](LICENSE) : domaine public, vous pouvez copier, modifier et redistribuer le projet sans condition.
