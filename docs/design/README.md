# Dossiers de conception de la refonte en Rust

Ces documents préparent la réécriture complète de Neon Hack en Rust (conception libre, moteur pur, deux frontends). Le plan d'exécution est [`docs/ROADMAP.md`](../ROADMAP.md) ; les spécifications normatives qui complètent ces dossiers sont dans [`docs/spec/`](../spec/README.md).

| Fichier | Contenu | Cité dans les documents comme |
|---|---|---|
| [`game-systems-audit.md`](game-systems-audit.md) | Catalogue chiffré des mécaniques du jeu en C, critique, trois directions de redesign, invariants, périmètre du moteur | « tâche A » |
| [`narrative-bible.md`](narrative-bible.md) | Inventaire de l'existant, univers, personnages, arc en trois actes, quêtes, textes à écrire, annexe des textes d'origine | « tâche B » |
| [`tui-and-accessibility.md`](tui-and-accessibility.md) | Contrat moteur ↔ frontends, frontend plain, TUI, spécification d'accessibilité (60 exigences), tests | « tâche C » |
| [`architecture-rust.md`](architecture-rust.md) | Workspace, API du moteur, contenu en données, i18n, sauvegarde, pyramide de tests | « tâche D » |
| [`delivery-and-ci.md`](delivery-and-ci.md) | Squelette R0, CI, releases (la migration vers `legacy-c/` est abandonnée : table rase du C) | « tâche E » |
| [`CROSS-CHECK.md`](CROSS-CHECK.md) | Contradictions entre les documents, trous, affirmations vérifiées | |
| [`DECISIONS.md`](DECISIONS.md) | Décisions du propriétaire, choix recommandés, décisions à valider | |

Les cinq premiers documents ont été écrits en parallèle par des agents indépendants le 7 octobre 2026, chacun avec un prototype jetable compilé sur `rustc 1.97.0`. Ces prototypes n'étaient pas dans le dépôt : les chemins `<scratchpad>` cités renvoient au dossier temporaire de la session. Les légendes `[E]` (exécuté), `[C]` (lu dans le code), `[D]` (connaissance non vérifiée), `[H]` (hypothèse) disent le degré de preuve de chaque affirmation.

**Règle de préséance** (`DECISIONS.md` § 4, R-0) : `DECISIONS.md` > `CROSS-CHECK.md` § 1 > les cinq dossiers. Les dossiers sont des **sources d'analyse non réécrites** : ils se contredisent sur 19 points, et les résolutions de `CROSS-CHECK.md` font foi. Elles s'appliquent dans le code, au lot qui touche chaque point.

**Références au code C.** Les dossiers citent des fichiers du jeu en C (`src/…`, `tests/…`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`) : le C est supprimé du dépôt (table rase, `DECISIONS.md` P3) mais reste lisible dans l'historique, par exemple `git show 653bc46:src/game/world.c`. Les références `fichier:ligne` s'entendent dans ce commit. La version d'origine non refondue est le tag distant `legacy-v2.087`.
