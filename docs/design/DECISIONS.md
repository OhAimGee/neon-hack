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
| R-6 | **Licences** (CROSS-CHECK 15) : on garde `directories` et on documente l'exception `option-ext` (MPL-2.0, licence de fichier, aucune modification, texte livré dans `THIRD-PARTY-LICENSES`) : une ligne de configuration contre ≈ 60 lignes de code à tester sur trois systèmes. **Vérifié en R0** (`cargo-deny` 0.20.2, `deny.toml` à la racine) : avec `directories` dans l'arbre, la licence passe avec l'exception et est rejetée sans elle. Le texte de licence part dans `THIRD-PARTY-LICENSES` à la release (R8). Repli : résolution maison |
| R-13 | **Glossaire** (lot P1.1) : le vocabulaire est un fichier (`data/glossary.toml`) plus un terme canonique par langue dans les catalogues (`term.<id>`), contrôlés par `cargo test` (concept sans terme, terme sans concept, deux concepts de même mot, variante interdite employée dans l'interface ; la narration n'est contrôlée que pour les concepts `strict`). Mécanisme mis en œuvre avec la spécification, qui a déjà trouvé une entorse dans le jeu jouet (« pseudo » au lieu de « handle »). La table de commandes du jeu complet est spécifiée dans `docs/spec/commands.md` ; son code attend R2 |
| R-12 | **Jeu jouet** (lot R1.5) : il sert à prouver le socle (contrat, textes, sauvegarde, commandes, réglages, couleurs, interfaces), pas à préfigurer le jeu. Objectif : acheter l'amélioration du deck (120 crédits) sans que la trace atteigne 100. Un scan rapporte 2 à 10 crédits et 5 à 25 de trace ; `laylow` retire 25 de trace ; le module de camouflage (60) divise par deux, arrondi au-dessus, la trace des scans ; le proxy (30) retire 10 de trace. Une fin (victoire ou défaite) **n'écrase pas l'autosave** : relancer le jeu reprend juste avant la fin, comme un « recharger la dernière sauvegarde » ; `--new` recommence. L'économie est validée par un joueur prudent écrit dans les tests (« joueur optimiste » du dossier d'architecture), qui doit gagner pour 300 graines |
| R-11 | **Couleurs** (lot R1.4c, COL-1 à COL-9 du dossier TUI) : décision de couleur dans l'ordre `--color`/`--no-color`, `NO_COLOR` non vide (une valeur vide est ignorée), `settings.toml`, détection ; un lecteur d'écran n'a jamais de séquence d'échappement. Sans couleur, la palette est `mono` (attributs seuls). Palettes : `default`, `high-contrast`, `cvd`, `mono` ; les deux palettes opaques peignent leur fond et leur texte atteint 4,5:1 (calculé par un test, pas affirmé) ; les couleurs exactes ne sont envoyées que si `COLORTERM` vaut `truecolor` ou `24bit`, sinon les couleurs nommées du terminal ; la palette `default` reste toujours en couleurs nommées pour suivre le thème (aucun rapport de contraste n'est revendiqué pour elle). Le plain s'en tient à 8 couleurs et au gras ; le transcript sans les séquences est identique. Le danger est toujours en vidéo inverse, l'erreur soulignée, l'avertissement en gras : la couleur n'est jamais la seule information. Aucun clignotement n'existe dans le modèle de style |
| R-10 | **Réglages** (lot R1.4b, CROSS-CHECK 18) : trois familles d'options (présentation, partie, développement). Précédence **par réglage**, du plus spécifique au plus général : langue = `--lang` > `NEON_HACK_LANG` > `settings.toml` > locale (`LC_ALL`, `LC_MESSAGES`, `LANG`) > anglais ; verbosité, ASCII et lecteur d'écran = ligne de commande > fichier > défaut, avec ASCII mis automatiquement par une locale qui nomme **explicitement** un autre jeu de caractères (`C` et `POSIX` n'y suffisent pas : les conteneurs les posent alors que le terminal fait de l'UTF-8). Une valeur d'environnement que le jeu ne connaît pas est ignorée. Variables préfixées `NEON_HACK_`, jamais d'`env` de clap sur un booléen. `settings.toml` vit dans le dossier de données ; un fichier abîmé ou trop gros (> 64 Kio) donne les défauts, un avertissement, et n'est **jamais réécrit** ; il n'est pas créé non plus (on l'écrit à la main pour l'instant : un écran de réglages vient avec le jeu réel). L'environnement est lu une fois et passé par valeur : les tests ne modifient jamais l'environnement du processus et lancent le vrai binaire avec un environnement vide. Couleurs et palettes : R1.4c |
| R-9 | **Sauvegarde, mise en œuvre** (lot R1.3, R-7 appliquée) : enveloppe TOML `version` / `[meta]` (nom, nombre d'actions : ce que montre une liste) / `[state]` / `[end] ok = true` ; le moteur ne touche pas au disque, il produit et lit du texte (`Game::snapshot`, constructeur du jeu concret : un chargement raté ne peut pas toucher une partie en cours) ; `Step::save` indique le genre (`Autosave`, `Checkpoint`, `Slot(n)`) et le frontend choisit le fichier ; 3 points de contrôle gardés (le plus récent d'abord), 9 emplacements ; une partie quittée est sauvegardée **à l'invite de commande** et une partie chargée n'est jamais terminée ; la démo crée un point de contrôle en entrant dans l'étal (achat irréversible) ; `--new` ne supprime rien (l'ancienne autosave devient `.bak`) ; un fichier illisible est mis de côté (`.corrupt`), jamais écrasé ni sauvegardé en `.bak` ; un échec d'autosave est signalé une fois, un échec manuel à chaque fois. Hors R1.3 : Hardcore, annulation, `settings.toml` et variables `NEON_HACK_*` (R1.4 et R2) |
| R-8 | **Textes** (lot R1.2, CROSS-CHECK 3) : catalogues TOML par langue dans `data/text/<langue>/*.toml`, tables imbriquées aplaties en clés pointées (une clé ne peut pas être à la fois un texte et une table) ; variantes `clé@sr` et `clé@ascii` ; gabarits `{nom}` et `{n|singulier|pluriel}` à formes imbriquables, accolades réservées et sans échappement (ce que tape le joueur passe en argument) ; règle de pluriel par langue (`Lang::plural`) ; `--ascii` = translittération complète de la ligne finale en un seul point, `?` pour ce qui n'a pas d'équivalent ; embarquement par `build.rs` (aucune E/S à l'exécution) ; contrôles de contenu `text::check` exécutés par les tests. Les budgets de largeur d'affichage attendent le schéma de contenu (R2) |
| R-7 | **Sauvegarde** (CROSS-CHECK 8) : trois notions distinctes, autosave courante, points de contrôle jamais écrasés par l'autosave, emplacements manuels (3 visibles, capacité 9) ; spécifiée avec le format en R1 |

## 5. Décisions proposées par les spécifications P1 (à valider par le propriétaire)

Chaque spécification de [`docs/spec/`](../spec/README.md) inscrit ici ce qu'elle tranche **par défaut** (moindre friction : appliqué tel quel sauf véto). Une décision du propriétaire la remplace sans refonte : le glossaire et la table de commandes sont des données.

| # | Spécification | Proposition |
|---|---|---|
| S-1 | [Glossaire](../spec/glossary.md) § 5 | Trace (intrusion) et Notoriété (campagne) remplacent « alerte » et « chaleur » ; site = carte du monde, nœud = graphe d'intrusion ; l'ICE mobile s'appelle patrouille (SENTINEL reste la division de Nexus) ; « niveau » à l'écran, « palier » jamais ; quête / contrat (jamais « quête annexe ») ; journal = journal de quêtes, terminal = panneau d'événements de la TUI ; *run* / intrusion ; handle ; message ; le réseau |
| S-2 | [Glossaire](../spec/glossary.md) § 6 (liée à D-09) | Les textes d'interface sont neutres (ni tu ni vous) ; chaque personnage garde sa voix |
| S-3 | [Commandes](../spec/commands.md) § 10 | `hack <site>` seule porte d'entrée d'une intrusion ; `use <programme>` pour tous les utilitaires ; nœuds par numéro ou nom (pas de lettre) ; `net` carte du monde, `map` graphe ; `talk` officiel et `contact` alias ; `journal` alias de `quests` ; `save` réservé au hub, autosave à chaque tour d'intrusion |
| S-4 | [Écran d'intrusion](../spec/run-screen.md) § 11 | Prévision détaillée au début du tour, résumée par une projection après les actions qui la changent ; elle n'annonce jamais l'aléa ni un programme inutilisable |
| S-5 | [Écran d'intrusion](../spec/run-screen.md) § 11 | `undo` annule le tour entier en cours (variante possible : l'action) |
| S-6 | [Langage de missions](../spec/missions.md) § 5 | Valider le langage tel que le spike l'exécute (14 objectifs, 9 conditions, 9 effets), la règle R-OPEN et les neuf corrections de la bible (§ 4 : trois impasses corrigées, dont M01 #5, la règle A contre la colonne « Palier », la réputation de M03) |
| S-7 | [Langage de missions](../spec/missions.md) § 5 | Pas de réacteur sur fait hors quête : « Neon Angel redevient libre » (bible 3.4) est retiré (option B) ; l'option A ajouterait un bloc `on_fact` |
| S-8 | [`neon-sim`](../spec/neon-sim.md) § 9 | Seuils d'équilibrage en données (`data/sim/targets.toml`), barrières P et objectifs O du § 5 comme point de départ, cadence de référence de 8 commandes par minute |
| S-9 | [Solveur](../spec/solver.md) § 7 | Le solveur exact est un outil de test et de CI ; à l'exécution, `AutoResolve` = exact borné (N = 5 000), puis repli `anytime`, puis Trace nominale du palier (« approximatif ») ; la prévision n'utilise pas le solveur |
| S-10 | [Solveur](../spec/solver.md) § 5 a | La marge de difficulté se calcule sur le kit minimal du palier, pas sur le kit complet |
| S-11 | [Solveur](../spec/solver.md) § 5 b-e | Spoof limité (pas sous la Trace du début du tour, ou une charge), « grillé » à tout instant, deck de 4 à 8 emplacements avec consommables hors deck, Virus et Overclock à rééquilibrer : décision de design la plus lourde de P1 (les utilitaires effacent la Trace dès le palier 3) |
