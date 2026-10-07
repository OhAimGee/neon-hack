# Feuille de route : refonte de Neon Hack en Rust

> Rédigée le 7 octobre 2026 à partir des cinq dossiers de [`docs/design/`](design/README.md) et de leur [cross-check](design/CROSS-CHECK.md), puis **adaptée à la table rase du C** (`design/DECISIONS.md` P3 et R-1). La feuille de route de la version en C disparaît avec elle ; elle reste lisible par `git show 653bc46:docs/ROADMAP.md`.
## Objectif

Une v1.0 jouable, **entièrement en Rust**, repartie de zéro, avec :

1. **une campagne complète** : début, milieu, fin, épilogue, au périmètre « min » (10 quêtes principales, 9 contrats, 19 quêtes, ≈ 25 graphes d'intrusion ; `DECISIONS.md` R-3) ;
2. **une TUI plein écran** avec panneau latéral, et un frontend « plain » équivalent ;
3. **une accessibilité intégrée dès le premier lot**, jamais ajoutée après coup (60 exigences testables du dossier TUI) ;
4. FR et EN complets, Linux, macOS et Windows natifs, binaires publiés.

Cadre : conception libre du jeu, moteur pur sans E/S, dépendances choisies, **aucun code ni aucune sauvegarde repris du C**.

## Architecture cible

```
crates/neon-engine   règles, état, événements, RNG, sérialisation (aucune E/S, ni horloge, ni HashMap)
crates/neon-cli      binaire neon-hack : options, dossiers, frontends plain et TUI
crates/neon-sim      harnais d'équilibrage (bots, métriques)
data/                structure du monde (data/world), textes (data/text/fr, en), glossaire
docs/                conception, décisions, feuille de route
```

Le moteur est une machine à états : `handle(Input) -> Step { events, prompt }`. Les événements portent un rôle sémantique et un texte `clé + arguments` ; la langue est résolue au rendu. Détail : [`architecture-rust.md`](design/architecture-rust.md) et [`tui-and-accessibility.md`](design/tui-and-accessibility.md), avec les résolutions du cross-check (qui prévalent sur les dossiers).

## Deux pistes parallèles

Le socle technique ne dépend pas du design de jeu et démarre tout de suite ; le moteur de jeu attend la validation des décisions de design restantes.

### Piste technique (démarre maintenant)

| Phase | Contenu | Fini quand |
|---|---|---|
| **R0** Table rase et squelette | **1. Fait.** Suppression du C et des fichiers parasites : `src/`, `tests/`, `Makefile`, les 3 scripts `demo_*.sh`, `docs/legacy/`, `docs/ARCHITECTURE.md`, l'ancien `docs/ROADMAP.md`, la CI du C ; ce fichier est devenu `docs/ROADMAP.md`. **2. Fait.** README FR et EN réécrits pour le projet Rust, `.gitignore` Rust. **3. Fait.** Workspace Cargo (édition 2024, résolveur 3, MSRV 1.88, **lints = union des deux jeux**, `deny.toml`, `rust-toolchain.toml`) avec trois talons : `neon-engine` (PCG32 et tests de propriété), `neon-cli`, `neon-sim`. **4. Fait.** CI sur 3 OS (fmt, clippy `-D warnings`, tests, MSRV, `cargo deny`, rustdoc, contrôle unique `CI OK`) ; le job plain-only arrive avec la feature `tui` (R3), le workflow de release en R8. **5. Fait.** Licences : exception `option-ext` / MPL-2.0 (R-6), vérifiée avec `cargo deny` | le dépôt ne contient plus que la licence, les README, `docs/`, le workspace et la CI ; CI verte sur les 3 systèmes ; `cargo deny` vert |
| **R1** Socle, en cinq lots (une PR chacun) | **R1.1 Fait.** Contrat moteur ↔ frontends unique (`neon-engine::{game, event, prompt, text}`, voir la rustdoc) compilé avec les deux frontends : `Step`, `Event`, `Prompt`, `Input` du dossier TUI plus `Input::{Cancel, Eof}`, `save_requested`, option indisponible = `Result<(), Text>`, fin de partie par `Prompt::End`, orateur = `ContactId`, `Text` à clé dynamique ; PCG32 ; jeu de démonstration (`--demo`) ; frontend plain et frontend TUI (modèle testé avec `TestBackend`, colle vérifiée dans un pseudo-terminal ; son test automatique arrive en R3) ; job CI plain-only. **R1.2** i18n (catalogues embarqués, pluriels, variantes `@sr` et `@ascii`, translittération ASCII, parité testée). **R1.3** sauvegarde (autosave, points de contrôle, emplacements, écriture atomique, migrations ; R-7). **R1.4** commandes et complétion ; réglages et CLI unifiés en trois familles (présentation, partie, développement) ; couleurs et `NO_COLOR`. **R1.5** un jeu jouet complet de bout en bout | un jeu jouet complet tourne en plain, FR et EN, avec sauvegarde et rechargement, et ses tests (unitaires, `proptest`, `insta`) passent sur 3 OS |

### Piste conception (en parallèle, bloque R2)

| Phase | Contenu | Fini quand |
|---|---|---|
| **P0** Cadrage | Dossiers de conception, cross-check, décisions et ce plan (fait : PR fusionnée) | fait |
| **P1** Spécifications restantes | **Registre unique des commandes** et **glossaire unique** (avant tout texte) ; spécification du **langage de missions** (+ spike sur M04-M05) ; spécification de l'**écran de run** et de sa linéarisation pour lecteur d'écran ; **second spike du solveur** (patrouilles, familles d'ICE, 8 nœuds) ; spécification de `neon-sim` (cibles chiffrées) ; validation par le propriétaire des décisions de design encore ouvertes (G1-G4, G6, G8-G11, D-04 à D-10). **Pas de réécriture des dossiers** (R-0) | spécifications écrites, spikes concluants, décisions inscrites dans `DECISIONS.md` |

## Phases de jeu (après R1 et P1)

| Phase | Contenu | Fini quand |
|---|---|---|
| **R2** Contenu et campagne | Schéma de contenu et validation (références, prérequis acycliques, parité FR/EN, budgets de largeur) ; langage de missions ; contacts, dialogues, courrier ; paliers de progression ; tutoriel et prologue écrits pour le nouveau jeu ; **toute la campagne jouable avec les runs en `AutoResolve`** ; textes lots L0-L1 (tranche verticale chapitres 1-2) | un e2e plain joue la campagne du périmètre retenu jusqu'à l'épilogue, sans impasse (joueur optimiste étendu aux drapeaux et aux décisions) |
| **R3** TUI | Frontend plein écran : journal, panneau (jauges nommées et chiffrées, quêtes, carte), saisie avec historique et TAB, écrans (boutique, contacts, courrier, quêtes, carte, deck, run), paliers de taille, bascule vers le plain ; accessibilité vérifiée par la checklist d'acceptation | tests `TestBackend` + `insta` et pty (`portable-pty` + `vt100`) verts, checklist d'accessibilité cochée |
| **R4** Moteur de run tactique | Graphe, programmes, ICE, Trace, **prévision**, annulation selon la difficulté ; solveur au run-time (`AutoResolve` = plan optimal, repli borné) ; préréglages de difficulté et mode histoire | chaque mission est prouvée terminable par le solveur ; `AutoResolve` remplacé par le tactique sans changer la campagne |
| **R5** Textes | Lots L2 à L5 puis L6 transversal ; relecture de voix ; relecture de l'anglais par un natif ; comptage par lot ; fins tranchées au lot L5 (R-4) | toutes les clés FR et EN présentes, budgets respectés, relectures faites |
| **R6** Équilibrage | `neon-sim` : bots, métriques, invariants (anti-farm, économie, difficulté monotone) ; cibles chiffrées validées avec le propriétaire | invariants verts, cibles atteintes |
| **R7** Durcissement | `proptest` nocturne, fuzz des chargeurs, mutation, test « singe », exécution réelle sous Windows et macOS, essai avec un lecteur d'écran réel | CI nocturne verte, protocole d'essai joué |
| **R8** Release v1.0 | Quatre binaires non signés + `SHA256SUMS` + attestation, `CHANGELOG`, README FR et EN à jour, `docs/ARCHITECTURE.md` écrit pour le Rust | tag `v1.0.0` publié |

**Ordre** : `R0 → R1` (et `P0 → P1` en parallèle) `→ R2 → R3 → R4 → R5 → R6 → R7 → R8`. Les textes (R5) avancent par tranches verticales dès R2 ; R3 peut démarrer dès que le contrat de R1 est stable.

## Volume estimé (ordres de grandeur, ± 40 %)

| Élément | Estimation |
|---|---|
| Moteur Rust | 9 500 à 14 800 lignes (+ tests du même ordre) |
| Frontend plain | 700 à 1 000 lignes |
| TUI | 2 000 à 3 000 lignes |
| `neon-sim` | 800 à 1 300 lignes |
| Texte à écrire | ≈ 27 k mots par langue au périmètre « min » (≈ 31 à 33 k avec le tutoriel et le prologue réécrits), soit ≈ 60 k mots FR + EN, contre ≈ 2 k aujourd'hui |

## Risques principaux

1. **Le moteur de run** (le plus gros chantier, complexité 5/5) n'a qu'un prototype jouet : d'où le second spike en P1 et la livraison en deux temps (`AutoResolve`, puis tactique).
2. **Le volume de texte** : sans relecture native de l'anglais ni outillage de comptage, la parité FR/EN dérive.
3. **Windows et macOS** n'ont été vérifiés qu'en compilation : le premier vrai run de la CI sera la première vérification.
4. **Repartir de zéro** : les prototypes des dossiers de conception n'ont pas été conservés ; le squelette, le contrat et le moteur sont écrits à neuf, à partir des extraits compilés des dossiers. C'est voulu (table rase) mais le coût est réel.
5. **Décisions de design encore ouvertes** (voir P1) : elles bloquent R2, pas R0 ni R1.

## Hors périmètre de la 1.0

Reprise de tout code ou de toute sauvegarde du C, audio (hors cloche terminale optionnelle, à décider), installeurs, Homebrew, winget, publication sur crates.io, signature des binaires, spécialisations de progression, extension de contenu au-delà du périmètre « min » (R-3).

## Règles de travail

- Petites PR par lot vers `main`, commit de fusion ; pré-versions `v0.N.0-alpha.K`.
- Chaque PR : `cargo fmt --check`, `clippy -D warnings`, tests sur 3 OS, `cargo deny`.
- Identifiants et commentaires de code en anglais ; docs et messages de commit en français.
- **Aucun test ne touche au dossier de données réel** (dossier temporaire obligatoire).
- Toute donnée persistante nouvelle passe par la sauvegarde avec un test d'aller-retour ; tout renommage d'id publié exige une migration.
- Les dossiers de `docs/design/` ne sont pas réécrits (R-0) : une contradiction se règle dans le code, au lot qui la touche.
