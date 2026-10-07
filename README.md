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
| Phase R1 : socle (contrat moteur ↔ frontends, i18n, sauvegarde, frontend plain) | **prochaine étape** |
| Moteur de jeu, TUI, campagne | à venir |

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
