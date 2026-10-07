# Feuille de route : refonte de Neon Hack en Rust

> Remplace [`ROADMAP.md`](ROADMAP.md) (feuille de route de la version en C, conservée pour mémoire). Rédigée le 7 octobre 2026 à partir des cinq dossiers de [`docs/design/`](design/README.md) et de leur [cross-check](design/CROSS-CHECK.md). Décisions : [`design/DECISIONS.md`](design/DECISIONS.md).

## Objectif

Une v1.0 jouable, **entièrement en Rust**, avec :

1. **une campagne complète** (début, milieu, fin, épilogue ; portée exacte à fixer, décision G5) ;
2. **une TUI plein écran** avec panneau latéral, et un frontend « plain » équivalent ;
3. **une accessibilité intégrée dès le premier lot**, jamais ajoutée après coup (60 exigences testables du dossier TUI) ;
4. FR et EN complets, Linux, macOS et Windows natifs, binaires publiés.

Cadre : conception libre du jeu (pas de parité avec le C), moteur pur sans E/S, dépendances choisies, aucun import des anciennes sauvegardes.

## Architecture cible

```
crates/neon-engine   règles, état, événements, RNG, sérialisation (aucune E/S, ni horloge, ni HashMap)
crates/neon-cli      binaire neon-hack : options, dossiers, frontends plain et TUI
crates/neon-sim      harnais d'équilibrage (bots, métriques)
data/                structure du monde (data/world), textes (data/text/fr, en), glossaire
legacy-c/            le jeu en C, jusqu'à sa suppression
```

Le moteur est une machine à états : `handle(Input) -> Step { events, prompt }`. Les événements portent un rôle sémantique et un texte `clé + arguments` ; la langue est résolue au rendu. Détail : [`architecture-rust.md`](design/architecture-rust.md) et [`tui-and-accessibility.md`](design/tui-and-accessibility.md), avec les résolutions du cross-check.

## Deux pistes parallèles

Le cross-check montre que **le socle technique ne dépend pas du design de jeu** et peut démarrer tout de suite, alors que le moteur de jeu attend la validation des décisions G1 à G12.

### Piste technique (démarre maintenant)

| Phase | Contenu | Fini quand |
|---|---|---|
| **R0** Squelette | Déplacement du C dans `legacy-c/` (script de migration vérifié : mêmes comptes de tests avant et après) ; workspace Cargo (édition 2024, résolveur 3, MSRV 1.88, lints unifiés, `deny.toml`, `rust-toolchain.toml`) ; CI sur 3 OS, job plain-only, `legacy-c` filtré par chemins ; décision T1 sur `directories` | CI verte sur les 3 systèmes, `make test` toujours vert dans `legacy-c/` |
| **R1** Socle | **Contrat moteur ↔ frontends unique**, compilé avec les deux frontends (CROSS-CHECK 1-2) ; PCG32 ; i18n (catalogue, pluriels, variantes `@sr`, parité testée) ; commandes et complétion ; sauvegarde (autosave, points de contrôle, emplacements, écriture atomique, migrations) ; réglages et CLI unifiés ; frontend plain minimal ; accessibilité de base (`NO_COLOR`, `--ascii`, mode lecteur d'écran) | un jeu jouet complet tourne en plain, FR et EN, avec sauvegarde et rechargement ; les 94 + 56 tests des prototypes repassent |

### Piste conception (en parallèle, bloque R2)

| Phase | Contenu | Fini quand |
|---|---|---|
| **P0** Cadrage | Les documents de conception, le cross-check et ce plan (cette PR) | PR relue et fusionnée |
| **P1** Consolidation | Le propriétaire tranche G1 à G12, D-01 à D-10 et T1-T3 ; résolution des 19 contradictions dans les documents ; **registre unique des commandes** et **glossaire unique** avant tout texte ; spécification du **langage de missions** (+ spike sur M04-M05) ; spécification de l'**écran de run** et de sa linéarisation pour lecteur d'écran ; **second spike du solveur** (patrouilles, familles d'ICE, 8 nœuds) ; spécification de `neon-sim` (cibles chiffrées) | décisions écrites dans `DECISIONS.md`, documents mis à jour, spikes concluants |

## Phases de jeu (après R1 et P1)

| Phase | Contenu | Fini quand |
|---|---|---|
| **R2** Contenu et campagne | Schéma de contenu et validation (références, prérequis acycliques, parité FR/EN, budgets de largeur) ; langage de missions ; contacts, dialogues, courrier ; paliers de progression ; tutoriel et prologue réécrits sur le nouveau jeu ; **toute la campagne jouable avec les runs en `AutoResolve`** ; textes lots L0-L1 (tranche verticale chapitres 1-2) | un e2e plain joue la campagne de la portée retenue jusqu'à l'épilogue, sans impasse (joueur optimiste étendu aux drapeaux et aux décisions) |
| **R3** TUI | Frontend plein écran : journal, panneau (jauges nommées et chiffrées, quêtes, carte), saisie avec historique et TAB, écrans (boutique, contacts, courrier, quêtes, carte, deck, run), paliers de taille, bascule vers le plain ; accessibilité vérifiée par la checklist d'acceptation | tests `TestBackend` + `insta` et pty (`portable-pty` + `vt100`) verts, checklist d'accessibilité cochée |
| **R4** Moteur de run tactique | Graphe, programmes, ICE, Trace, **prévision**, annulation selon la difficulté ; solveur au run-time (`AutoResolve` = plan optimal, repli borné) ; préréglages de difficulté et mode histoire | chaque mission est prouvée terminable par le solveur ; `AutoResolve` remplacé par le tactique sans changer la campagne |
| **R5** Textes | Lots L2 à L5 puis L6 transversal ; relecture de voix ; relecture de l'anglais par un natif ; comptage par lot | toutes les clés FR et EN présentes, budgets respectés, relectures faites |
| **R6** Équilibrage | `neon-sim` : bots, métriques, invariants (anti-farm, économie, difficulté monotone) ; cibles chiffrées validées avec le propriétaire | invariants verts, cibles atteintes |
| **R7** Durcissement | `proptest` nocturne, fuzz des chargeurs, mutation, test « singe », exécution réelle sous Windows et macOS, essai avec un lecteur d'écran réel | CI nocturne verte, protocole d'essai joué |
| **R8** Release v1.0 | Quatre binaires non signés + `SHA256SUMS` + attestation, `CHANGELOG`, README FR et EN, `docs/ARCHITECTURE.md` réécrit pour le Rust, **suppression de `legacy-c/`** (tag `legacy-c-final`) | tag `v1.0.0` publié |

**Ordre** : `R0 → R1` (et `P0 → P1` en parallèle) `→ R2 → R3 → R4 → R5 → R6 → R7 → R8`. Les textes (R5) avancent par tranches verticales dès R2 ; R3 peut démarrer dès que le contrat de R1 est stable.

## Volume estimé (ordres de grandeur, ± 40 %)

| Élément | Estimation |
|---|---|
| Moteur Rust | 9 500 à 14 800 lignes (+ tests du même ordre) |
| Frontend plain | 700 à 1 000 lignes |
| TUI | 2 000 à 3 000 lignes |
| `neon-sim` | 800 à 1 300 lignes |
| Texte à écrire | ≈ 27 à 33 k mots par langue (périmètre « min » à « cible ») contre ≈ 2 k aujourd'hui, soit ≈ 58 à 66 k mots FR + EN |

## Risques principaux

1. **Le moteur de run** (le plus gros chantier, complexité 5/5) n'a qu'un prototype jouet : d'où le second spike en P1 et la livraison en deux temps (`AutoResolve`, puis tactique).
2. **Le volume de texte** : sans relecture native de l'anglais ni outillage de comptage, la parité FR/EN dérive.
3. **Windows et macOS** n'ont été vérifiés qu'en compilation : le premier vrai run de la CI sera la première vérification.
4. **Le périmètre de la campagne** n'est pas fixé (G5) : chaque quête ajoute des runs à construire, pas seulement du texte.
5. **Les prototypes ne sont pas dans le dépôt** : à archiver avant de les porter, sous peine de les refaire.

## Hors périmètre de la 1.0

Audio (hors cloche terminale optionnelle, à décider), installeurs, Homebrew, winget, publication sur crates.io, signature des binaires, spécialisations de progression, extension de contenu au-delà de la portée retenue, import des anciennes sauvegardes.

## Règles de travail

- Petites PR par lot vers `main`, commit de fusion ; pré-versions `v0.N.0-alpha.K`.
- Chaque PR : `cargo fmt --check`, `clippy -D warnings`, tests sur 3 OS, `cargo deny`, et `make test` dans `legacy-c/` tant qu'il existe.
- Identifiants et commentaires de code en anglais ; docs et messages de commit en français.
- **Aucun test ne touche au dossier de données réel** (dossier temporaire obligatoire).
- Toute donnée persistante nouvelle passe par la sauvegarde avec un test d'aller-retour ; tout renommage d'id publié exige une migration.
