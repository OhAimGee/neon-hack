# Dossiers de conception de la refonte en Rust

Ces documents préparent la réécriture complète de Neon Hack en Rust (conception libre, moteur pur, deux frontends). Le plan d'exécution est [`docs/ROADMAP-RUST.md`](../ROADMAP-RUST.md).

| Fichier | Contenu | Cité dans les documents comme |
|---|---|---|
| [`game-systems-audit.md`](game-systems-audit.md) | Catalogue chiffré des mécaniques du jeu en C, critique, trois directions de redesign, invariants, périmètre du moteur | « tâche A » |
| [`narrative-bible.md`](narrative-bible.md) | Inventaire de l'existant, univers, personnages, arc en trois actes, quêtes, textes à écrire, annexe des textes d'origine | « tâche B » |
| [`tui-and-accessibility.md`](tui-and-accessibility.md) | Contrat moteur ↔ frontends, frontend plain, TUI, spécification d'accessibilité (60 exigences), tests | « tâche C » |
| [`architecture-rust.md`](architecture-rust.md) | Workspace, API du moteur, contenu en données, i18n, sauvegarde, pyramide de tests | « tâche D » |
| [`delivery-and-ci.md`](delivery-and-ci.md) | Migration vers `legacy-c/`, squelette R0, CI, releases | « tâche E » |
| [`CROSS-CHECK.md`](CROSS-CHECK.md) | Contradictions entre les documents, trous, affirmations vérifiées | |
| [`DECISIONS.md`](DECISIONS.md) | Décisions du propriétaire, choix recommandés, décisions à valider | |

Les cinq premiers documents ont été écrits en parallèle par des agents indépendants le 7 octobre 2026, chacun avec un prototype jetable compilé sur `rustc 1.97.0`. Ces prototypes n'étaient pas dans le dépôt : les chemins `<scratchpad>` cités renvoient au dossier temporaire de la session. Les légendes `[E]` (exécuté), `[C]` (lu dans le code), `[D]` (connaissance non vérifiée), `[H]` (hypothèse) disent le degré de preuve de chaque affirmation.

**Les documents se contredisent sur 19 points** (voir `CROSS-CHECK.md`) : en cas de conflit, la résolution retenue dans `CROSS-CHECK.md` prévaut jusqu'à la passe de consolidation (phase P1 de la feuille de route).
