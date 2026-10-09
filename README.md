# 🌆 Neon Hack

RPG textuel cyberpunk pour le terminal. Vous êtes un hacker novice à Neo-Tokyo, en 2087, face à la mégacorporation Nexus Corp et à son mystérieux Projet Aurora.

> Jeu généré par IA en 2025 et laissé inachevé, puis refondu une première fois en C. Il est maintenant **réécrit entièrement en Rust, en repartant de zéro**.

**Français** · [English](README.en.md)

## Statut

**La campagne se joue de bout en bout, du nouveau jeu à l'épilogue, mais le jeu n'est pas terminé.** C'est le jeu lancé par défaut (`neon-hack`) : prologue, tutoriel guidé, commandes du hub, cinq fins et épilogue, FR et EN, avec les intrusions résolues automatiquement (le moteur tactique est la phase R4). Les textes des chapitres 1 et 2 sont écrits ; ceux des chapitres 3 à 6, des fins et de l'épilogue sont encore des brouillons `TODO <clé>` (502 par langue). La démonstration du moteur reste accessible avec `--demo`. Le cadrage, le socle (phase R1), les spécifications (phase P1) et le contenu et la campagne (phase R2) sont terminés ; l'interface plein écran (R3) a son cœur (R3.1 : saisie, défilement, paliers de taille, terminal toujours rendu) ; son panneau latéral complet, ses écrans et ses réglages sont les lots R3.2 et R3.3.

| Étape | État |
|---|---|
| Cadrage et dossiers de conception (systèmes de jeu, narration, TUI et accessibilité, architecture, CI) | fait : [`docs/design/`](docs/design/README.md) ; les spécifications détaillées sont dans [`docs/spec/`](docs/spec/README.md) et les décisions de design sont validées |
| Feuille de route de la v1.0 | écrite : [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Ancien code C et fichiers parasites | supprimés (voir [Ancienne version](#ancienne-version-en-c)) |
| Phase R0 : squelette Cargo, CI sur trois systèmes | fait |
| Phase R1.1 : contrat moteur ↔ frontends, deux interfaces (plain et plein écran), jeu de démonstration | fait |
| Phase R1.2 : textes en TOML embarqués, pluriels, `--ascii` strictement 7 bits, contrôles de parité FR/EN | fait |
| Phase R1.3 : sauvegarde (autosave, points de contrôle, emplacements, copie de secours, versions de format) | fait |
| Phase R1.4a et R1.4b : registre des commandes, réglages (`settings.toml`, variables d'environnement, trois familles d'options) | fait |
| Phase R1.4c : couleurs, `NO_COLOR`, palettes (`default`, `high-contrast`, `cvd`, `mono`) | fait |
| Phase R1.5 : jeu jouet complet de bout en bout (objectif, victoire, défaite, épilogue) | fait |
| Phase R1 (socle) | **terminée** |
| Phase P1 : spécifications (glossaire, commandes, langage de missions, écran d'intrusion, solveur, banc d'équilibrage) et décisions de design validées | **terminée** ; prochaine étape : le contenu et la campagne (R2) |
| Phase R2.1 : moteur de contenu et données de la campagne (quêtes, décisions, fins, validation, sauvegarde de l'état) | fait |
| Phase R2.2 : registre de commandes du jeu complet (R2.2a) et cœur de la campagne, commandes du hub, intrusions automatiques, sauvegarde (R2.2b) | fait |
| Phase R2.3 : textes du lot L1 (chapitres 1 et 2), prologue et tutoriel guidé | fait |
| Phase R2.4 : campagne de bout en bout (joueur optimiste par les commandes, 216 plans, cinq fins), campagne par défaut | fait |
| Phase R2 (contenu et campagne) | **terminée** |
| Phase R3.1 : cœur de la TUI (choix du frontal et question du premier lancement, saisie avec historique et TAB, journal défilant, paliers 64×20 et 100×28, terminal rendu par toutes les sorties, tests dans un pseudo-terminal) | fait |
| Panneau latéral, écrans, commandes du frontal (R3.2 et R3.3), intrusion tactique (R4), textes des chapitres 3 à 6 (R5), équilibrage (R6) | à venir |

## Jouer en local

> **Le jeu n'est pas fini** : la campagne est jouable de bout en bout, mais ses textes des chapitres 3 à 6 sont des brouillons et les intrusions se résolvent automatiquement. Il n'y a pas de binaire précompilé avant la v1.0 : on compile le projet soi-même, ce qui prend de quelques dizaines de secondes à quelques minutes la première fois (téléchargement des dépendances compris).

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

### 3. Lancer le jeu

```bash
cargo run --release -p neon-cli
```

La première fois, Cargo télécharge et compile les dépendances ; les lancements suivants sont immédiats. Pour obtenir une commande `neon-hack` utilisable depuis n'importe où :

```bash
cargo install --path crates/neon-cli --locked
neon-hack
```

`rustup` a déjà placé `~/.cargo/bin` dans le `PATH`. Pour mettre à jour : avec un clone, `git pull` puis relancez la commande ci-dessus (avec `--force` pour `cargo install`) ; avec le ZIP, téléchargez-le de nouveau. Pour désinstaller : `cargo uninstall neon-cli`.

Sans option, `neon-hack` joue la **campagne** : il reprend la dernière partie, ou en commence une. `neon-hack --demo` lance à la place la démonstration du moteur, un mini-jeu qui a ses propres sauvegardes (une sauvegarde de l'un n'est jamais lue par l'autre : le jeu le dit).

### 4. Comment jouer

Dans un vrai terminal assez grand, le jeu s'ouvre en **plein écran** (barre d'état, journal, panneau latéral). Si l'entrée ou la sortie est redirigée, si le terminal est trop petit, ou avec `--plain`, il se joue **ligne par ligne**. Au tout premier lancement sur un terminal, une question demande le mode d'affichage (plein écran, ligne par ligne, lecteur d'écran) et l'enregistre dans `settings.toml`.

- Au début : un pseudo (*Entrée* garde `Neon`), le prologue (*Entrée* pour continuer, `skip` pour passer), puis l'offre du **tutoriel guidé** (`o` ou `n`).
- But : suivre les **quêtes** du journal (`quests`), parler aux contacts (`talk`), pirater des sites (`hack`), acheter (`shop`, `buy`), garder sa **Notoriété** basse (`laylow`), et décider. `help` liste ce qui est possible à ce moment-là ; `hint` demande un indice à ECHO-7.
- Menus : tapez le **numéro** d'une entrée ou son nom ; `0` ou une ligne vide revient en arrière (*Échap* en plein écran).
- Sauvegarde : la partie est **sauvegardée automatiquement** et reprise au lancement suivant (`--new` pour recommencer) ; `save` ou `save 2` l'écrit dans un emplacement (1 à 9) ; le jeu crée un **point de contrôle** à l'ouverture de chaque quête principale, avant une décision et avant un achat (les 3 derniers sont gardés). `neon-hack --list-saves` montre le dossier et ce qu'il contient.
- Plein écran : *TAB* complète la commande ou l'argument sous le curseur (et liste ce qui reste ambigu), *↑*/*↓* rappellent les commandes déjà tapées, *Ctrl+A/E/U/K/W* et les flèches éditent la ligne, *PgUp*/*PgDn* (et *Début*/*Fin* quand la ligne est vide) font défiler le journal, *Échap* revient en bas. *Ctrl+C* demande à quitter, comme la commande `quit` ; *Ctrl+D* sur une ligne vide ferme l'entrée ; une touche ferme ensuite l'interface. Un collage de plusieurs lignes devient une seule ligne, qui n'est jamais exécutée sans *Entrée*. La souris n'est pas capturée (la sélection et la copie du terminal marchent).
- La démonstration (`--demo`) : acheter l'**amélioration du deck** (120 crédits) à l'étal de R4Z0R sans que la **trace** atteigne 100 (`scan`, `laylow`, `shop`).

| Option | Effet |
|---|---|
| `--plain` | interface ligne par ligne, même sur un terminal |
| `--tui` | interface plein écran, et un mot sur `stderr` si le terminal ne peut pas la montrer |
| `--screen-reader` | mode lecteur d'écran : interface ligne par ligne, sans symboles à épeler |
| `--ascii` | ASCII 7 bits : symboles, accents et texte tapé translittérés, sans décoration |
| `--lang fr` / `--lang en` | langue des textes (anglais par défaut) |
| `--verbosity brief\|normal\|full` | quantité d'ambiance affichée |
| `--demo` | joue la démonstration du moteur à la place de la campagne (sauvegardes à part, dossier `saves/`) |
| `--difficulty story\|normal\|expert\|hardcore` | difficulté d'une nouvelle campagne (une campagne reprise garde la sienne) |
| `--seed 7` | partie reproductible (pour une nouvelle partie) |
| `--color auto\|always\|never`, `--no-color` | couleur : un terminal qui sait la montrer par défaut ; `NO_COLOR` l'éteint, et ces options l'emportent |
| `--palette default\|high-contrast\|cvd\|mono` | couleurs de l'interface plein écran |
| `--print-settings` | affiche les réglages en vigueur et d'où chacun vient |
| `--new` | recommence au lieu de reprendre la sauvegarde automatique (l'ancienne devient `auto.toml.bak`) |
| `--load auto\|checkpoint-1\|slot-2` | reprend cette sauvegarde (`checkpoint-1` à `checkpoint-3`, `slot-1` à `slot-9`) |
| `--list-saves` | liste les sauvegardes et le dossier où elles sont |
| `--no-save` | joue sans rien écrire |
| `--data-dir DIR` | dossier des sauvegardes, à la place de celui du système |

L'interface plein écran demande au moins **64×20** caractères, et **100×28** pour afficher le panneau latéral permanent. Au lancement, un terminal plus petit donne l'affichage ligne par ligne avec une ligne d'explication sur `stderr` ; en cours de partie, le terminal rétréci affiche seulement « Terminal trop petit » et la partie reste intacte jusqu'à son agrandissement. Un terminal `TERM=dumb` ou un mode lecteur d'écran n'ouvre jamais le plein écran.

Les sauvegardes sont dans le dossier de données de l'utilisateur, sous-dossier `saves-campaign` (`saves` pour la démonstration) : `~/.local/share/neon-hack` sous Linux, `~/Library/Application Support/neon-hack` sous macOS, `%APPDATA%\neon-hack\data` sous Windows. Une sauvegarde abîmée n'est jamais écrasée : le jeu reprend la copie précédente (`.bak`) et le dit, ou explique comment recommencer.

### Réglages

Les options de présentation (`lang`, `verbosity`, `ascii`, `screen_reader`, `display`) se fixent une fois pour toutes dans un fichier `settings.toml`, dans le dossier de données (le dossier parent de celui que montre `--list-saves`, qui est `saves-campaign`). Le jeu ne crée ce fichier que pour la question du premier lancement (clé `display`, et `screen_reader` pour le mode lecteur d'écran) et ne modifie jamais un fichier qui existe ; le reste s'écrit à la main :

```toml
lang = "fr"
verbosity = "brief"
ascii = false
screen_reader = false
display = "auto"   # auto, tui ou plain
```

Pour chaque réglage, la source la plus précise gagne : la ligne de commande, puis la variable d'environnement (`NEON_HACK_LANG` pour la langue), puis `settings.toml`, puis la configuration du système (`LANG` pour la langue ; une locale qui nomme un autre jeu de caractères que l'UTF-8, comme `fr_FR.ISO-8859-1`, active `--ascii`), puis la valeur par défaut. `NEON_HACK_DATA_DIR` change le dossier de données. Côté couleurs : `color` (`auto`, `always`, `never`) et `palette` se règlent aussi dans `settings.toml` (`NEON_HACK_PALETTE` pour la palette) ; la variable standard `NO_COLOR` éteint la couleur (la palette `mono` n'emploie que gras, soulignement et vidéo inverse), mais pas si `--color` est donné ; le mode lecteur d'écran n'envoie jamais de séquence d'échappement. `high-contrast` (blanc sur noir) et `cvd` (couleurs adaptées aux daltonismes) peignent leur fond ; leurs couleurs exactes demandent un terminal 24 bits (`COLORTERM=truecolor`), sinon le terminal en donne l'approximation. Un fichier abîmé est signalé, n'est jamais modifié, et les valeurs par défaut s'appliquent. `neon-hack --print-settings` montre le résultat.

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
