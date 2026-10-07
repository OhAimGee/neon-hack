# Registre des décisions

Trois niveaux : **décisions du propriétaire** (fermes), **choix techniques recommandés** (retenus provisoirement, véto possible en relisant la PR), **décisions à valider** (bloquantes avant le moteur de jeu). Les recommandations viennent des cinq dossiers de conception ; les renvois (§) pointent dans ces dossiers. Les contradictions entre dossiers sont dans [`CROSS-CHECK.md`](CROSS-CHECK.md).

## 1. Décisions du propriétaire (7 octobre 2026)

| # | Sujet | Décision |
|---|---|---|
| P1 | Ambition | **Refonte libre du design de jeu dès le départ** : pas de parité avec le C, qui reste une référence de lore, de mécaniques et d'invariants |
| P2 | Interface | **Deux frontends** sur un moteur pur : « plain » (tubes, tests, lecteurs d'écran) et TUI plein écran avec panneau latéral |
| P3 | Code C | Déplacé dans `legacy-c/` puis supprimé quand le Rust couvre ses fonctionnalités |
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
| Livraison D5 | Suppression de `legacy-c/` (tag `legacy-c-final`) quand **l'e2e plain joue la campagne de la portée retenue jusqu'à l'épilogue** et que la TUI est jouable |
| Livraison D6 | Réglages GitHub à appliquer par le propriétaire : protection de `main` sur le contrôle `CI OK`, règle de tags `v*`, jeton en lecture seule, Dependabot |

## 3. Décisions à valider avant le moteur de jeu (phase R2)

### 3.1 Design de jeu (audit § 6)

| # | Question | Recommandation |
|---|---|---|
| G1 | Modèle de piratage | **A. Infiltration à tours** (graphes d'ICE visibles, Trace, résolution déterministe), livrée en deux temps : campagne complète en `AutoResolve`, puis moteur tactique. Repli : B, contrats à jets lisibles |
| G2 | Place du hasard | Déterministe, menaces annoncées ; aléa limité au butin et aux variantes |
| G3 | Échec | Pas de game over par défaut (« grillé » = rejouer le run avec pénalité de Notoriété) + mode Hardcore |
| G4 | Progression | Paliers 1 à 6 liés aux quêtes principales, plus d'XP par action ; spécialisations après la 1.0 |
| G5 | Envergure | **Une seule portée à fixer** (CROSS-CHECK 5). Recommandation : périmètre « min » de la bible (10 principales, 9 contrats, 19 quêtes, ≈ 25 graphes) ; extension planifiée ensuite |
| G6 | Contacts | 9 contacts avec rôle de jeu, arc de 2-3 scènes et confiance gagnée par des choix |
| G7 | Embranchements et fins | Modèle de la bible (3 décisions, 4 fins de fond + variante d'ECHO-7), E2 « Harmonie » supprimable (CROSS-CHECK 6) |
| G8 | Mode histoire | Résolution automatique d'un run, option explicite, sans pénalité, marquée dans la sauvegarde |
| G9 | Vitesse du texte | Réglage de présentation interruptible ; le moteur n'attend jamais (mesuré : 790 s d'attente cumulée par partie avec les animations actuelles) |
| G10 | Saisie | Numéros **et** noms avec TAB ; plus aucune chaîne à recopier ni mot secret |
| G11 | Annulation d'un tour | Oui en Histoire (illimité) et Normal (3 par run), non en Expert et Hardcore |
| G12 | Canon d'ECHO-7 | Version de la bible (septième copie numérique d'un humain mort) par défaut (CROSS-CHECK 7) ; un véto suffit, avant le lot L2 |

### 3.2 Narration (bible § 0.4)

D-01 vérité de fond (ECHO-7, AURA, le Courtier) · D-02 périmètre (cible 23 quêtes, repli 19) · D-03 décisions et fins · D-04 renommages (« Agent Smith » → Elias Voss ; Phase 3 « Ghost Protocol » → Zénith ; implants = Halcyon) · D-05 ton et notes de contenu sensible au premier lancement · D-06 conséquences permanentes avec instantané avant D1-D3 · D-07 documents chiffrés sans saisie · D-08 organisation des textes (structure neutre séparée des textes par langue) · D-09 voix (tutoiement dans l'underground, vouvoiement pour AURA, le Courtier et Voss) · D-10 ordre d'écriture (tranche verticale chapitres 1-2 d'abord). **Recommandation du document : oui à toutes**, avec les replis indiqués dans la bible ; D-02 et D-03 sont en conflit avec l'audit (G5, G7), voir ci-dessus.

### 3.3 Technique

| # | Question | Recommandation |
|---|---|---|
| T1 | Dossiers de données : `directories 6.0.0` tire `option-ext` sous MPL-2.0, que le `deny.toml` refuse | Exception documentée (`deny.toml` et `about.toml`, texte de licence dans `THIRD-PARTY-LICENSES`) **ou** résolution maison (≈ 60 lignes) ; le propriétaire choisit |
| T2 | Troisième langue envisagée ? | Non pour la 1.0 : i18n maison. Oui : Fluent dès maintenant |
| T3 | Appliquer les réglages GitHub (livraison D6) | Oui, par le propriétaire : ils ne se font pas depuis le dépôt |
