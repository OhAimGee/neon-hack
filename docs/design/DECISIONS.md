# Registre des décisions

Trois niveaux : **décisions du propriétaire** (fermes), **choix techniques recommandés** (retenus provisoirement, véto possible en relisant la PR), **décisions à valider** (bloquantes avant le moteur de jeu). Les recommandations viennent des cinq dossiers de conception ; les renvois (§) pointent dans ces dossiers. Les contradictions entre dossiers sont dans [`CROSS-CHECK.md`](CROSS-CHECK.md).

## 1. Décisions du propriétaire (7 octobre 2026)

| # | Sujet | Décision |
|---|---|---|
| P1 | Ambition | **Refonte libre du design de jeu dès le départ** : pas de parité avec le C, qui reste une référence de lore, de mécaniques et d'invariants |
| P2 | Interface | **Deux frontends** sur un moteur pur : « plain » (tubes, tests, lecteurs d'écran) et TUI plein écran avec panneau latéral |
| P3 | Code C | **Table rase** (décision du 7 octobre 2026, après la première) : le C est supprimé dès la phase R0 au lieu d'être déplacé dans `legacy-c/`. Son historique reste dans git : dernier état du C = commit `653bc46`, version d'origine = tag `legacy-v2.087` (distant). Voir § 4, R-1 |
| P4 | Anciennes sauvegardes C | **Pas d'import** |
| P5 | Dépendances | Pragmatiques et choisies (`clap`, `serde`, `ratatui`, `thiserror`, `proptest`…) |
| P6 | Priorités | **1. Campagne complète, 2. TUI, 3. Accessibilité** |
| P7 | Histoire | Libre d'évoluer et de s'étoffer ; **l'assistant écrit les textes manquants** |
| P8 | Valeurs par défaut | Validées : Rust stable édition 2024 ; Linux, macOS et Windows natifs ; contenu embarqué et validé par tests ; i18n maison FR/EN avec parité testée ; sauvegarde texte versionnée via `serde` ; identifiants et commentaires de code en anglais ; docs en français (README aussi en anglais) ; messages de commit en français |

## 2. Choix techniques recommandés (retenus provisoirement)

| Source | Choix |
|---|---|
| Architecture D1 | Moteur = **machine à états pure** (`handle(Input) -> Outcome`), menus et conversations en états empilés ; ni `trait Ui` bloquant, ni coroutine |
| Architecture D2 | **TOML partout** (contenu, textes, réglages, sauvegardes), embarqué et validé en tests et au démarrage ; JSON réservé aux rapports de simulation |
| Architecture D3 | Moteur **pur** : ni horloge, ni E/S, ni `HashMap` (imposé par `clippy`) ; PCG32 maison ; temps = compteur de tours |
| Architecture D4 | Politique de sauvegarde à **reprendre** : voir CROSS-CHECK 8 (autosave / points de contrôle / emplacements manuels) |
| Architecture D5, livraison D1 | MSRV **1.88** (plancher de `ratatui 0.30.2`), compilateur épinglé en version exacte, `tui` par défaut, construction plain-only vérifiée en CI |
| Architecture D6 | i18n maison pour FR/EN ; **Fluent seulement si une 3e langue à pluriels riches est prévue** (à dire maintenant) |
| Architecture D7 | Identifiants textuels stables ; contenu invalide = échec explicite au démarrage ; renommage d'un id publié = migration |
| Architecture D8 | Un run de piratage est un `Flow::Run` derrière une interface (`AutoResolve` d'abord, tactique ensuite) |
| TUI 1 | Langue résolue **au rendu** (clé + arguments typés), variantes `@sr` pour lecteurs d'écran |
| TUI 2 | Éditeur de saisie **maison** pour la TUI (`tui-input` en plan B) ; plain « cuit », sans TAB |
| TUI 3 | Paliers 64×20 (compact) et 100×28 (panneau permanent) ; repli plain au démarrage |
| TUI 4 | TUI si entrée et sortie sont des terminaux assez grands, sinon plain ; question de mode au premier lancement |
| TUI 5 | Quatre palettes (`default`, `high-contrast`, `cvd`, `mono` forcée par `NO_COLOR`) |
| TUI 6 | Souris **désactivée** par défaut ; collage bracketé toujours actif |
| TUI 7 | `--ascii` strict (7 bits), distinct du mode lecteur d'écran |
| TUI 8 | Pas de dépendance directe à `crossterm` (ré-export de `ratatui`) ; tests pty en `cfg(unix)` ; `cargo check` Windows et macOS dès le premier lot |
| Livraison D2, D3 | Release par **matrice écrite à la main** (pas `cargo-dist`) ; v1.0 = 4 binaires (Linux x86_64 statique, macOS arm64 et x86_64, Windows x86_64) + `SHA256SUMS` + attestation, non signés, ni installeur ni crates.io |
| Livraison D4 | Petites PR par lot vers `main`, commit de fusion, pré-versions `v0.N.0-alpha.K` |
| Livraison D5 | **Sans objet** (table rase, voir P3). Le critère « un e2e plain joue la campagne de la portée retenue jusqu'à l'épilogue » devient le critère de fin de la phase R2 |
| Livraison D6 | Réglages GitHub à appliquer par le propriétaire : protection de `main` sur le contrôle `CI OK`, règle de tags `v*`, jeton en lecture seule, Dependabot |

## 3. Décisions à valider avant le moteur de jeu (phase R2)

### 3.1 Design de jeu (audit § 6)

| # | Question | Recommandation |
|---|---|---|
| G1 | Modèle de piratage | **A. Infiltration à tours** (graphes d'ICE visibles, Trace, résolution déterministe), livrée en deux temps : campagne complète en `AutoResolve`, puis moteur tactique. Repli : B, contrats à jets lisibles |
| G2 | Place du hasard | Déterministe, menaces annoncées ; aléa limité au butin et aux variantes |
| G3 | Échec | Pas de game over par défaut (« grillé » = rejouer le run avec pénalité de Notoriété) + mode Hardcore |
| G4 | Progression | Paliers 1 à 6 liés aux quêtes principales, plus d'XP par action ; spécialisations après la 1.0 |
| G5 | Envergure | **Tranché par défaut** (§ 4, R-3) : périmètre « min » de la bible (10 principales, 9 contrats, 19 quêtes, ≈ 25 graphes) ; extension planifiée ensuite |
| G6 | Contacts | 9 contacts avec rôle de jeu, arc de 2-3 scènes et confiance gagnée par des choix |
| G7 | Embranchements et fins | **Reporté au lot L5** (§ 4, R-4) : hypothèse de travail = modèle de la bible (3 décisions, 4 fins de fond + variante d'ECHO-7), E2 « Harmonie » supprimable sans refonte |
| G8 | Mode histoire | Résolution automatique d'un run, option explicite, sans pénalité, marquée dans la sauvegarde |
| G9 | Vitesse du texte | Réglage de présentation interruptible ; le moteur n'attend jamais (mesuré : 790 s d'attente cumulée par partie avec les animations actuelles) |
| G10 | Saisie | Numéros **et** noms avec TAB ; plus aucune chaîne à recopier ni mot secret |
| G11 | Annulation d'un tour | Oui en Histoire (illimité) et Normal (3 par run), non en Expert et Hardcore |
| G12 | Canon d'ECHO-7 | **Tranché par défaut** (§ 4, R-5) : version de la bible (septième copie numérique d'un humain mort) ; véto possible jusqu'au lot L2 |

### 3.2 Narration (bible § 0.4)

D-01 vérité de fond (ECHO-7, AURA, le Courtier) · D-02 périmètre (cible 23 quêtes, repli 19) · D-03 décisions et fins · D-04 renommages (« Agent Smith » → Elias Voss ; Phase 3 « Ghost Protocol » → Zénith ; implants = Halcyon) · D-05 ton et notes de contenu sensible au premier lancement · D-06 conséquences permanentes avec instantané avant D1-D3 · D-07 documents chiffrés sans saisie · D-08 organisation des textes (structure neutre séparée des textes par langue) · D-09 voix (tutoiement dans l'underground, vouvoiement pour AURA, le Courtier et Voss) · D-10 ordre d'écriture (tranche verticale chapitres 1-2 d'abord). **Recommandation du document : oui à toutes**, avec les replis indiqués dans la bible ; D-02 et D-03 étaient en conflit avec l'audit (G5, G7) : résolu en § 4 (R-3, R-4).

### 3.3 Technique

| # | Question | Recommandation |
|---|---|---|
| T1 | Dossiers de données : `directories 6.0.0` tire `option-ext` sous MPL-2.0, que le `deny.toml` refuse | **Tranché par défaut** (§ 4, R-6) : exception documentée (`deny.toml` et `about.toml`, texte de licence dans `THIRD-PARTY-LICENSES`) ; repli : résolution maison (≈ 60 lignes) |
| T2 | Troisième langue envisagée ? | Non pour la 1.0 : i18n maison. Oui : Fluent dès maintenant |
| T3 | Appliquer les réglages GitHub (livraison D6) | Oui, par le propriétaire : ils ne se font pas depuis le dépôt |

Restent à valider **sans bloquer R0, R1 ni P1** : G1 à G4, G6, G8 à G11 et D-04 à D-10. Recommandation : oui à tout, avec véto possible à la relecture de chaque lot.

## 4. Résolutions par défaut, « moindre friction » (7 octobre 2026)

Le propriétaire a demandé la solution la moins frictionnante pour les contradictions du [cross-check](CROSS-CHECK.md). Principe : **ne pas réécrire 400 Ko de dossiers** ; poser une règle de préséance, trancher par défaut ce qui bloque, reporter le reste au moment où cela bloque, et corriger dans le code, qui ne peut pas se contredire puisqu'il compile.

| # | Résolution |
|---|---|
| R-0 | **Préséance** : `DECISIONS.md` > `CROSS-CHECK.md` § 1 (résolutions) > les cinq dossiers. Les dossiers sont des **sources d'analyse** : on ne les réécrit pas, on n'y ajoute une note de correction que si une erreur d'implémentation en découlerait |
| R-1 | **Table rase du C** (P3) : suppression de `src/`, `tests/`, `Makefile`, des 3 scripts de démonstration, de `docs/legacy/`, de `docs/ARCHITECTURE.md`, de `docs/ROADMAP.md` (feuille de route du C) et de la CI du C, en première action de R0. Raisons : la conception est libre, donc aucune parité à tenir ; les mécaniques, chiffres et textes d'origine sont déjà extraits dans les dossiers (annexe A de la bible pour les textes) ; garder le C imposerait migration, double CI et filtres de chemins pour un code que personne ne fera évoluer ; l'historique reste dans git (`git show 653bc46:<chemin>`). Abandonnés par conséquent : la migration vers `legacy-c/` (livraison § 1), le job `legacy-c.yml` et le critère de suppression du C |
| R-2 | Les contradictions **techniques** (CROSS-CHECK 1-4, 9-14, 16-18) **se règlent dans le code, au lot qui les touche** : le contrat moteur ↔ frontends est écrit une seule fois et compilé avec les deux frontends au premier lot de R1 (base : `Step`, `Event`, `Prompt`, `Input` du dossier TUI) ; les lints sont l'union des deux jeux dès le squelette R0 ; le registre unique des commandes et le glossaire unique précèdent le premier texte (phase P1). Aucune passe de réécriture des dossiers |
| R-3 | **Portée de la v1.0** (CROSS-CHECK 5) = périmètre « min » de la bible : 10 quêtes principales, 9 contrats d'avantage, 19 quêtes, ≈ 25 graphes d'intrusion, ≈ 27 k mots par langue. Les quêtes M04, M08, M11, M12 et les 7 contrats « de couleur » sont une extension planifiée, après la campagne jouable en `AutoResolve`. Justification : c'est la plus petite campagne *complète*, donc la moindre friction pour livrer la priorité 1, et elle s'étend sans refonte |
| R-4 | **Fins** (CROSS-CHECK 6) : aucune décision avant le lot L5. Hypothèse de travail : modèle de la bible (trois décisions). Rien d'écrit avant L5 n'en dépend, hormis les drapeaux `narr.*` déjà prévus |
| R-5 | **Canon d'ECHO-7** (CROSS-CHECK 7, D-01) : version de la bible, l'histoire ayant été confiée à l'assistant (P7) et les indices s'écrivant à partir du lot L2. Véto possible jusqu'à L2 ; la décision 12 de l'audit est caduque |
| R-6 | **Licences** (CROSS-CHECK 15) : on garde `directories` et on documente l'exception `option-ext` (MPL-2.0, licence de fichier, aucune modification, texte livré dans `THIRD-PARTY-LICENSES`) : une ligne de configuration contre ≈ 60 lignes de code à tester sur trois systèmes. À vérifier avec `cargo deny` en R0 ; repli : résolution maison |
| R-7 | **Sauvegarde** (CROSS-CHECK 8) : trois notions distinctes, autosave courante, points de contrôle jamais écrasés par l'autosave, emplacements manuels (3 visibles, capacité 9) ; spécifiée avec le format en R1 |
