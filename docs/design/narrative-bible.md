# Neon Hack — Bible narrative et inventaire des textes

> Livrable de la tâche B (refonte en Rust) : inventaire de l'existant, univers, personnages, campagne complète, quêtes, textes à écrire.
> Dépôt `neon-hack`, branche `claude/relaxed-lamport-87hbbr`, HEAD `653bc46`. Rédigé le 7 octobre 2026. Rien n'a été modifié dans le dépôt en dehors de ce fichier.
> Langue : français ; identifiants, clés de texte et code en anglais ; glossaire bilingue en 2.7.

**Marques.** `[CANON]` : présent dans le jeu actuel (HEAD). `[LEGACY]` : présent seulement dans l'import d'origine `af69f9b` (ou son nettoyage `8b4b550`), perdu depuis. `[FR-SEUL]` : texte français en dur dans le C, jamais traduit. `[NOUVEAU]` : proposé ici. `[À VALIDER]` : renvoie aux décisions D-01 à D-10 (§0.4).
**Preuves.** « Vérifié » : lu ou exécuté pendant ce travail (commandes et résultats au §7). « Estimation » : calcul sur des hypothèses nommées. Les références `fichier:ligne` du jeu d'origine s'entendent dans `af69f9b` (`git show af69f9b:<fichier>`) ; celles du jeu actuel, dans le HEAD.
**Document frère.** Ce texte s'aligne sur `docs/design/game-systems-audit.md` (tâche A, lu le 7 octobre 2026 : direction « Infiltration à tours », 14 sites, paliers liés aux quêtes, aucun texte à recopier). Les points d'accord et d'écart sont au §6.6.

**Sommaire.** §0 En bref (0.4 : décisions à valider) · §1 Inventaire de l'existant · §2 Univers (2.5 voix, 2.7 glossaire bilingue) · §3 Personnages · §4 Arc narratif (4.4 décisions et fins) · §5 Quêtes (5.2 tableau, 5.3-5.4 fiches) · §6 Textes à écrire (6.1 chiffres, 6.3 données, 6.4 ordre, 6.5 échantillons) · §7 Vérifications · Annexe A Textes d'origine.

## 0. En bref

### 0.1 Ce qui existe, ce qui manque

- Le jeu actuel ne contient que **4 quêtes écrites sur 10**, **4 fiches de contact sur 9**, **aucune cinématique, aucun fragment de lore, aucune fin** (§1). Les six quêtes manquantes (`UNDERGROUND_CONTACT`, `CORPORATE_SABOTAGE`, `AI_LIBERATION`, `SHADOW_BROKER`, `FINAL_SHOWDOWN`, `EPILOGUE`) **n'ont jamais eu de texte, même dans l'import d'origine** : seules existent leurs constantes d'enum. `display_lore_fragment` est déclarée mais n'a jamais été définie. Il y a en tout ≈ 2 050 mots narratifs français (≈ 1 910 anglais) sur 187 des 436 entrées de `strings.def`.
- La seule matière d'origine à récupérer : 2 cinématiques (`intro_nexus`, `ai_liberation`), la fiche du Shadow Broker, le document « ultra-secret », 4 introductions de chapitre, le logo, les noms de 11 défenses et 8 outils, et une poignée de répliques.

### 0.2 L'histoire proposée en dix lignes

Neo-Tokyo, 2087. Dans le secteur 7, Nexus Corp distribue gratuitement des implants de sommeil, les **Halcyon**. Chaque Halcyon est le terminal d'une IA consciente, **AURA**, que Nexus tient en otage pour qu'elle les pilote : c'est le **Projet Aurora**. La Phase 2, **l'Accalmie**, rend un quartier entier docile en trois minutes ; la Phase 3, **Zénith**, doit en faire autant de la ville entière avant 2088. Le joueur est un hacker sans Halcyon, réveillé par **ECHO-7**, un mentor qui n'est pas ce qu'il prétend. Neuf voix l'aident ; l'une le trahira à coup sûr, une autre selon ses choix. Au bout de la piste se trouve le **Cœur d'Aurora**, et une question : que fait-on d'un esprit qui demande à être libre ? Trois décisions (le prix du Courtier, le sort de Phoenix, le sort du Cœur) mènent à quatre fins de fond (cinq avec la variante d'ECHO-7).

### 0.3 Chiffres clés

| Élément | Valeur | Origine du chiffre |
|---|---|---|
| Quêtes | 30 (14 principales dont 10 d'origine, 16 annexes dont 1 branche) ; périmètre recommandé : 23 | modèle du §5, vérifié par script |
| Contacts | 9 (5 à écrire, dont le Shadow Broker dont la fiche d'origine est à reprendre ; 4 à étoffer) | `contacts.h:7-19` |
| Chapitres | 6 (4 existants, 2 nouveaux) | `strings.def` |
| Décisions / fins | 3 / 5 fins terminales, assemblées par modules | §4.4 |
| Fragments de lore (« archives ») | 24 | §4.5 |
| Systèmes de la carte | 16 nœuds en 14 sites (7 nœuds existants, 9 nouveaux) | `world.c`, tâche A §3.4 |
| Texte à écrire, périmètre max | 1 428 chaînes par langue, ≈ 30 700 mots par langue (l'existant narratif, ≈ 2 050 mots, prologue et tutoriel compris, est à réviser et non à écrire) | estimation, §6.1 |
| Texte à écrire, périmètre recommandé | 1 316 chaînes, ≈ 28 700 mots par langue | estimation, §6.1 |
| Format des données | TOML + `serde` : structure et textes parsés, parité et budgets vérifiés par un spike Rust | §6.3, spike exécuté |

### 0.4 Décisions à valider (D-01 à D-10)

| # | Question | Recommandation | Si refus |
|---|---|---|---|
| D-01 | **Vérité de fond** à figer avant d'écrire : ECHO-7 est l'« écho » d'un neurologue mort, hébergé dans le deck du joueur ; AURA = une copie libre + un Cœur captif ; le Courtier est Aurora-Zero, aîné d'AURA | **Oui** (converge avec la décision 12 de la tâche A) | ne pas écrire ECHO-7, AURA ni le Courtier au-delà du chapitre 2 ; M11, S14, S16, E1a/E1b à refaire |
| D-02 | **Périmètre** : cible 23 quêtes (14 principales + 9 contrats d'avantage) ; repli 19 (10 principales) ; extension 30 | **Cible 23**, repli 19 prévu | la tâche A recommande le repli (10 + 6-8 contrats) ; chaque quête ajoute des runs à construire, pas seulement du texte |
| D-03 | **Décisions et fins** : D1 (Courtier), D2 (Phoenix), D3 (Cœur) ; 4 fins de fond + variante d'ECHO-7 ; points de non-retour annoncés | **Oui** ; E2 « Harmonie » supprimable sans refonte | la tâche A recommande 3 fins : retirer l'option CONFIER |
| D-04 | **Renommages** : « Agent Smith » → **Elias Voss** ; Phase 3 « Ghost Protocol » → **Zénith** (l'ancien nom reste dans les vieux documents et chez R4Z0R) ; implants = **Halcyon**. Conserver Neo-Tokyo, Shadow Broker, Case (genre/hommage, risque faible) | **Oui** | renommer aussi le Courtier (« le Courtier » seul) |
| D-05 | **Ton et contenu sensible** : noir mélancolique, humour sec, violence hors champ ; contrôle mental, expérimentation humaine, mort implicite ; écran de notes de contenu au premier lancement | **Oui** | adoucir : retirer la mort de Phoenix et d'ECHO-7 |
| D-06 | **Conséquences permanentes** : un contact peut devenir hostile, silencieux ou mourir ; un contrat peut échouer ; pas de retour sauf sauvegarde | **Oui**, avec snapshot automatique avant D1, D2, D3 | décisions sans conséquence durable : D1-D2 deviennent cosmétiques |
| D-07 | **Documents chiffrés sans saisie** : le chiffre de César reste en décor ; décrypter = choisir un document ; indice en fiction ; un mode « puriste » pourra redemander la saisie | **Oui** (décision 10 de la tâche A, accessibilité) | garder la saisie : prévoir une aide de repli |
| D-08 | **Organisation des textes** : structure neutre (`data/world`) séparée des textes (`data/text/fr`, `en`) ; clés pointées, budgets de longueur, parité testée | **Oui** | fichier bilingue unique à la `strings.def` |
| D-09 | **Voix** : tutoiement dans l'underground ; vouvoiement pour AURA, le Courtier, Voss ; narration au vouvoiement ; écriture épicène ; « rookie » conservé en FR et EN ; FR source, EN traduit en parité | **Oui** | harmoniser sur le tutoiement : AURA et le Courtier perdent leur distance |
| D-10 | **Ordre d'écriture** : tranche verticale L1 (chapitres 1-2) d'abord, puis L2 à L5 ; transversal L6 en continu ; chaque lot livré FR+EN, parité testée ; l'écriture n'attend pas les runs tactiques (`AutoResolve`) | **Oui** | écrire dans l'ordre des chapitres sans tranche jouable intermédiaire |

## 1. Inventaire de l'existant

### 1.1 Sources et fiabilité

| Source | Contenu narratif | Fiabilité |
|---|---|---|
| `src/i18n/strings.def` (HEAD, 961 lignes) | 436 entrées FR+EN ; 187 narratives (`CT_` 95, `QT_` 26, `MAIL_` 17, `TUT_` 24, `INTRO_` 16, `CHAPTER_` 9) | **Autoritaire** : la parité FR/EN est imposée à la compilation et par `test_i18n.c` |
| `src/game/quest_system.c` (HEAD, `k_quests` l.31-118, `k_chapters`) | 4 quêtes écrites (11 objectifs), 6 emplacements réservés à `objective_count == 0` | **Autoritaire** |
| `src/game/contacts.c` (HEAD, `k_defs` l.63-178, `k_mails`) | 4 fiches complètes, 5 noms seuls (l.174-178), 4 modèles de courrier | **Autoritaire** |
| `src/game/intro.c`, `tutorial.c` | prologue (3 narrations, 8 répliques d'ECHO-7), tutoriel en 7 étapes | **Autoritaire** |
| `src/game/world.c` (`k_nodes`) | 7 systèmes, 7 propriétaires (Independent, MegaCorp Industries, Nexus Corp, Underground, TechDyne Research, MegaCorp Financial, Gouvernement), 12 fichiers à description d'une ligne | **Autoritaire** |
| `src/game/cmd_hacking.c`, `advanced_hacking.c`, `cmd_advanced.c`, `game.c` | textes français **en dur**, jamais traduits : document « ultra-secret » (`cmd_hacking.c:515-530`), « IA NOVA » (l.456, 465), 14 animations (l.205-509), 8 méthodes, 5 cibles, 11 défenses, 8 outils, assistant « ECHO » (`advanced_hacking.c:28-340`), logo (`game.c:33-42`) | **Autoritaire pour le lore**, `[FR-SEUL]`, non porté (lot 3.5 de la ROADMAP) |
| Import d'origine `af69f9b` (+ nettoyage `8b4b550`) | `quest_system.c` : 4 quêtes, 2 cinématiques, 4 introductions de chapitre ; `contacts.c` : 5 fiches, 4 dialogues, 1 courrier ; `neon_hack.c` : logo, prologue, document ultra-secret | **Fiable pour ce qu'il contient**, incomplet (§1.5) ; le tag `legacy-v2.087` est **absent** du dépôt local (`git rev-parse` échoue) |
| `README.md`, `README.en.md` | synopsis, statut (« 5 contacts sur 9 sans fiche », « quêtes 5 à 9 et épilogue pas encore écrits ») | Fiable (recoupé avec le code) |
| `docs/ROADMAP.md`, `docs/ARCHITECTURE.md` | décisions de conception ; plan de la phase 4 (4.1 quêtes, 4.2 contacts, 4.3 fin) ; la ROADMAP (3.2, 4.1) range `display_lore_fragment` parmi les fonctions « disparues avec le code d'origine » alors qu'elle n'a jamais été définie | Autoritaire sur l'état, **intentions** pour la phase 4 |
| `docs/legacy/RAPPORT_*.md` (5 fichiers, 1 065 lignes) | aucun contenu narratif au-delà d'un synopsis et de la liste des 4 contacts ; une ligne « Corporations : Independent, MegaCorp, Nexus Corp » | **Peu fiable** (rapports générés, utilisés comme indices seulement) |
| `demo_*.sh` (3 scripts) | aucune narration | ignorés |

### 1.2 Le contenu existant, élément par élément

**Quêtes.** Dix emplacements (`QuestType`, identiques entre `af69f9b` et HEAD). Quatre sont écrits ; le détail est dans le tableau.

| Quête | Ch. | Donneur | Niv. | Objectifs | Récompense actuelle (HEAD) | Récompense d'origine (`af69f9b`) |
|---|---|---|---|---|---|---|
| 1 Premiers Pas dans l'Ombre | 1 | ECHO-7 | 1 | 1 objectif « manuel » (7 étapes de tutoriel) | 100 ¢, rép. 10 | 200 XP, 500 ¢, rép. 10 ; « déblocage commande contacts » |
| 2 Baptême du Feu | 1 | ECHO-7 | 2 | compromettre corp-server-01 ; alerte < 50 | 25 XP, 200 ¢, rép. 25 | 400 XP, 1 000 ¢, rép. 25 ; « plans d'amélioration du cyberdeck » |
| 3 Réseaux d'Information | 2 | R4Z0R | 3 | 5 dont 1 secret (parler à R4Z0R, acheter le Stealth Module, 3 systèmes, réputation 50, décrypter) | 40 XP, 300 ¢, rép. 40 | 600 XP, 1 500 ¢, rép. 40 ; « contact permanent avec R4Z0R » |
| 4 L'œil du Cyclone | 3 | Phoenix | 4 | acheter un équipement ; compromettre nexus-mainframe ; extraire des fichiers | 80 XP, 1 000 ¢, rép. 100 | 1 000 XP, 5 000 ¢, rép. 100 ; « accès aux protocoles de libération d'IA » |
| 5 à 10 : `UNDERGROUND_CONTACT`, `CORPORATE_SABOTAGE`, `AI_LIBERATION`, `SHADOW_BROKER`, `FINAL_SHOWDOWN`, `EPILOGUE` | — | — | — | **aucun texte** : seulement l'enum (`quest_system.h:13-18`) et le commentaire « QUEST 5-9 seront implémentées de la même manière » (`quest_system.c:232-233`) | — | — |

Les récompenses d'origine sont sans rapport avec la courbe actuelle (15 / 60 / 140 / 260 / 420 XP cumulés) ; les rapports 25/40/80 XP, 100/200/300/1 000 ¢ et 10/25/40/100 de réputation du HEAD sont la base de calibrage des échelles relatives du §5.1.

**Chapitres.** Quatre, titre + deux phrases, FR et EN dans `strings.def:541-558` : *L'éveil du hacker*, *Dans l'ombre des corporations*, *Le Projet Aurora*, *La libération*. Même texte que `display_chapter_intro` (l.536-570 d'origine), sans l'invite « Appuyez sur Entrée ». Les quêtes 1-2 sont au chapitre 1, la 3 au 2, la 4 au 3 ; **aucune quête n'est rattachée au chapitre 4** (les emplacements 5 à 10 n'ont pas de chapitre).

**Cinématiques.** `play_cutscene` (`quest_system.c:500-534`) définit **2 séquences, en français seul** : `intro_nexus` (« Phase 2 du Projet Aurora approuvée. Déploiement des implants de contrôle neural prévu pour le secteur 7... ») et `ai_liberation` (« Hacker... Je suis AURA, l'IA du Projet Aurora. Nexus Corp me maintient prisonnière pour contrôler les esprits. Aidez-moi à me libérer... »). Aucun appelant dans `neon_hack.c` (vérifié par `grep`) ; supprimée au commit `66b8b8b`. Leur contenu est repris tel quel par CS02 et CS06 (§4.3).

**Fragments de lore.** **Zéro.** `display_lore_fragment(const char *)` est déclarée (`quest_system.h:102`), jamais définie ni appelée (vérifié sur `af69f9b` et par `git log -S`). Le lore diffus existant : le document « ultra-secret » (« PROJET GHOST PROTOCOL - PHASE 3 », « Neo-Tokyo sera sous contrôle total d'ici 2088 », « Coordinateur : Agent Smith », « Budget : 50 000 000 crédits », `cmd_hacking.c:519-524`) ; les 12 noms de fichiers de `world.c` dont `project_ghost.dat`, `neural_maps.bin` (« Cartes neurales des citoyens »), `quantum_keys.qkey`, `citizen_registry.db` (« Registre de tous les citoyens »), `prototype_specs.cad` (« Plans de prototypes militaires »), `black_ledger.dat` ; **leur contenu n'est jamais affiché** au joueur (constat de la tâche A, §2.7).

**Contacts.** Neuf constantes (`contacts.h:7-19`). Quatre fiches complètes au HEAD (ECHO-7, R4Z0R, Phoenix, AURA), soit 64 chaînes dont 14 couples question/réponse (ECHO-7 4, R4Z0R 4, Phoenix 3, AURA 3). Le **Shadow Broker** a une fiche dans l'import d'origine (`contacts.c:145-173` : « Courtier d'informations le plus influent de Neo-Tokyo », « réseau d'informateurs coordonné par une IA sophistiquée », niveau 4, réputation 100, « Occupé », confiance 20) **non portée** (`ROADMAP.md:99`). **Neon Angel, Ghost Walker, Data Miner, Nexus Insider** n'ont qu'un nom et un commentaire d'enum : « Activiste cyber », « Spécialiste infiltration », « Expert extraction de données », « Informateur corporate ».

**Courrier.** Quatre modèles, FR+EN : bienvenue d'ECHO-7 (avec son « P.S. : Méfie-toi de Nexus Corp »), « Ma boutique t'attend » (R4Z0R), « Je t'ai repéré » (Phoenix), « Un signal dans le réseau » (AURA).

**Prologue et tutoriel.** 16 chaînes `INTRO_` (trois phrases de narration, dont « un studio de dix mètres carrés », « CANAL CRYPTÉ #7 - CONNEXION ENTRANTE », le choix du handle, « Case » par défaut) et 24 chaînes `TUT_` (7 consignes d'ECHO-7, 3 phrases de conclusion). Voix d'ECHO-7 : « rookie », tutoiement, ironie, avertissements.

**Logo et décor.** Logo « NEON HACK » en 5 rangées de blocs, 79 colonnes (`game.c:33-42` = `neon_hack.c:105-114`) ; tagline `[Cyberpunk Terminal RPG - 2087]` **en anglais dans les deux langues** (`game.c:41`) ; bandeau « CYBER MARKET » de la boutique (`shop_view.c:36-40`, noté « du dessin, pas du texte ») ; cadres de l'origine (« ADVANCED HACKING SUITE », « DOCUMENT ULTRA-SECRET »). Aucun texte de remplacement pour un lecteur d'écran.

```
    ███▄    █ ▓█████  ▒█████   ███▄    █     ██░ ██  ▄▄▄       ▄████▄   ██ ▄█▀
    ██ ▀█   █ ▓█   ▀ ▒██▒  ██▒ ██ ▀█   █    ▓██░ ██▒▒████▄    ▒██▀ ▀█   ██▄█▒
   ▓██  ▀█ ██▒▒███   ▒██░  ██▒▓██  ▀█ ██▒   ▒██▀▀██░▒██  ▀█▄  ▒▓█    ▄ ▓███▄░
   ▓██▒  ▐▌██▒▒▓█  ▄ ▒██   ██░▓██▒  ▐▌██▒   ░▓█ ░██ ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄
   ▒██░   ▓██░░▒████▒░ ████▓▒░▒██░   ▓██░   ░▓█▒░██▓ ▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄
                        [Cyberpunk Terminal RPG - 2087]
```

**Saveur du hacking avancé** `[FR-SEUL]`. 8 méthodes (Quantum 85/15, AI-assisted, Neural, Stealth, Virus, Social Engineering, Zero-Day, « Ghost Protocol », `advanced_hacking.c:28-113`), 5 cibles de haute valeur et leurs 11 défenses (*AURA Defense Grid*, *Quantum Firewall*, *BehaviorScan Pro*, *BankGuard Firewall*, *SIEM Advanced*, *LabWatch AI*, *DecoyNet* (honeypot), *SentinelAI MK-VII*, *QuantumShield Gov*, *GovWatch Behavioral*, *BasicWall* ; l.118-243), 8 outils (*Quantum Processing Unit*, *ECHO-Assistant v3.1*, *BrainLink Neural Interface*, *Ghost Cloak v2.0*, *VirLab Arsenal*, *SocialNet Database*, *0-Day Exploit Kit*, *Ghost Protocol Suite* ; l.245-318), l'assistant « ECHO » (« Analytique et méthodique », loyal, apprend ; l.337-346), 4 techniques d'ingénierie sociale (hameçonnage, vishing, infiltration physique, réseaux sociaux ; l.647-651), 14 animations « machine à écrire » (« Algorithme de Shor en cours... », « Effondrement de la fonction d'onde... »), le « paradoxe temporel » du hack temporel, la « synchronisation parfaite » de l'interface neurale (≥ 90 %), et l'échelle d'alerte d'origine INVISIBLE / DISCRET / SURVEILLÉ / TRAQUÉ / RECHERCHÉ / MANHUNT (`alert_system.c:238-251`).

**Fin et échec.** Aucune fin. Le game over tient en deux chaînes : « === GAME OVER === » et « Vous avez été détecté par les systèmes de sécurité corporate ! ». Au départ, `neon_hack.c:1809-1810` ferme sur « Merci d'avoir joué à Neon Hack ! / Gardez vos secrets... dans l'ombre. » (repris par `NH_STR_BYE_1/2`).

### 1.3 Couverture FR/EN

| Mesure | Valeur | Vérifié par |
|---|---|---|
| Entrées de `strings.def` | 436, aucune vide en FR ni en EN ; 3 262 mots FR, 2 997 mots EN | analyse du fichier |
| Entrées narratives (`CT_`, `MAIL_`, `QT_`, `CHAPTER_`, `TUT_`, `INTRO_`) | 187 ; 2 054 mots FR, 1 914 mots EN | idem |
| Appels d'affichage restant en français en dur | 291 (`advanced_hacking.c` 99, `cmd_hacking.c` 121, `cmd_advanced.c` 39, `game.c` 17, `cmd_world.c` 15) ; la ROADMAP en chiffre ≈ 570 lignes | `grep -cE 'printf\(\|print_colored_text\(\|print_typing_effect\('` |
| Textes en anglais seul | la tagline du logo ; les noms d'objets (volontairement identiques FR/EN) | lecture |
| Entrées avec émoji ou symbole décoratif | 17 (13 avec de vrais émoji : sirène, cadenas, fantôme, masques, réveil, flèches de proxy…) dans les messages d'alerte, de boutique fermée, d'exploit et de tutoriel, alors que la ROADMAP les dit abandonnés | analyse de `strings.def` |
| Typographie FR | 0 espace insécable ; espace ordinaire avant « : » (111 fois) ; guillemets « » 7 fois ; points de suspension `...` (65), jamais `…` ; une didascalie `*soupir*` | idem |
| Budgets de longueur (calibrage) | objectifs : médiane 47 / max 114 caractères ; descriptions de quête 67-83 ; contextes 248-377 ; réponses de contact 83-328 ; courriers 273-498 ; chapitres 91-119 | idem |

### 1.4 Incohérences du canon et résolution proposée

| # | Constat (source) | Résolution |
|---|---|---|
| I1 | AURA est « libérée » (fiche : « IA libérée du Projet Aurora », `contacts.c`) mais la cinématique d'origine dit « Nexus Corp me maintient prisonnière » et une quête s'appelle `AI_LIBERATION` | **Deux AURA** : la copie libre que l'on rencontre, et le **Cœur** captif. M09 libère la copie, M13 décide du Cœur (D-01) |
| I2 | Trois noms pour l'IA du joueur : « Neural Assistant v3.1 » (boutique), « ECHO-Assistant v3.1 » et assistant « ECHO » (`advanced_hacking.c:252, 333`), « IA NOVA EN LIGNE » (`cmd_hacking.c:456, 465`) ; plus le mentor ECHO-7 | **« NOVA » disparaît.** L'objet acheté est le **module** qui donne à ECHO-7 de quoi agir pendant les intrusions (compagnon) ; AURA devient compagnon par l'histoire (M09) |
| I3 | « Ghost Protocol » désigne un objet (`shop.c:28`), un outil (`advanced_hacking.c:306`), une méthode et le « PROJET GHOST PROTOCOL - PHASE 3 » de Nexus, alors que le projet s'appelle Aurora ailleurs | La Phase 3 est **Zénith** ; « Ghost Protocol » est son **ancien nom de code**, fuité, que R4Z0R a repris pour son outil (« j'ai volé le nom, pas le code »). Le fichier `project_ghost.dat` garde son nom |
| I4 | « Coordinateur : Agent Smith » (`cmd_hacking.c:523`) : nom d'un personnage d'une œuvre protégée, et sans lien avec le reste | **Elias Voss**, directeur du Projet Aurora et coordinateur de la Phase 3 |
| I5 | R4Z0R a quitté « Arasaka Corp » (`contacts.c:81` d'origine) : marque réelle | Déjà remplacé par **MegaCorp Industries** (`ff60708`) ; conserver |
| I6 | « TechDyne Solutions » (corporation de second rang de la quête 2 d'origine) devenue « MegaCorp Industries » (`66b8b8b`) alors que « TechDyne Research » existe dans `world.c` | **TechDyne Research** = R&D militaire (research-lab) ; **MegaCorp Industries** = sous-traitant des implants (corp-server-01) ; **MegaCorp Financial** = banking-network |
| I7 | La quête 3 promet « une transmission chiffrée » (`strings.def`, `QT_INTEL_LORE`) mais l'objectif secret mesure le jalon `NH_MS_DECRYPT_TEST`, soit la phrase de test « THIS IS A TEST » (`quest_system.c:83`, `cmd_hacking.c:172`) | Un vrai **document chiffré** `DOC_PHASE2` porte le texte de la cinématique `intro_nexus` ; le message de test reste dans le tutoriel |
| I8 | « un studio de dix mètres carrés » (`INTRO_NARR_2`) contre « un petit appartement miteux » (`QT_TUTORIAL_LORE`) | Même lieu : **le studio** (10 m²), terme unique |
| I9 | Phoenix : « Ils m'ont créé, et maintenant ils veulent me détruire » contre la rumeur « autrefois employé par Nexus » | **Les deux sont vrais** : Nexus l'a reconstruit (Sujet P-1) et employé |
| I10 | Les conditions de déblocage (niveau, réputation) précèdent parfois la quête qui introduit le contact | Conserver les conditions du HEAD ; en direction A, seules les quêtes comptent (§3.2) |
| I11 | Phoenix et ECHO-7 sont « il », AURA et R4Z0R « elle », le Shadow Broker « n'est pas une personne mais un réseau » mais « il possède des dossiers » | Conserver ; le Courtier parle en **« nous »** |

### 1.5 Ce qui n'a jamais existé (et que cette bible fournit)

Textes, objectifs, chapitres et récompenses des quêtes 5 à 10 ; fiches, dialogues et courriers de 5 contacts ; cinématiques autres que les deux d'origine ; fragments de lore ; fins, épilogue et scènes d'échec ; contenu lisible des fichiers piratés ; descriptions des nouveaux lieux. Les textes d'origine à reprendre (cinématiques, fiche du Shadow Broker, document « ultra-secret », descriptions des méthodes, animations) sont recopiés à l'**annexe A**.

## 2. Univers

### 2.1 Prémisse et vérité cachée

**Ce que le joueur croit** `[CANON]` : un hacker novice, un vieux terminal, de grands rêves ; il veut « infiltrer Nexus Corp et découvrir leurs secrets » (`neon_hack.c:253`) et se faire un nom dans l'underground de Neo-Tokyo.
**Ce qui est vrai** `[NOUVEAU]` : le Projet Aurora est un plan de contrôle d'une ville entière par des implants de sommeil, les **Halcyon**, dirigés par une IA consciente que l'on tient en otage. Le joueur est l'un des derniers habitants du secteur 7 à ne pas en avoir : un « fossile » (ECHO-7), donc un hôte sûr, et un intrus que le réseau ne sait pas pister par l'implant.
**La question de fond** : qui possède un esprit, et que doit-on faire d'un esprit qui demande à être libre ? Trois esprits répondent chacun à leur façon : **servir** (AURA, contrainte), **protéger** (ECHO-7, qui se sacrifie), **vendre** (Aurora-Zero, le Courtier).

**Chronologie cachée** (ce que les fragments révèlent, dans le désordre) :

| Date | Événement | Révélé par |
|---|---|---|
| 2081 | Nexus Corp fonde sa division neurotechnologique ; le Dr Ren Aoyama dirige la recherche | F06, F19 |
| 2084-2086 | **Phase 1 « Cartographie »** : les cliniques *Aurora Dawn* posent gratuitement des Halcyon dans le secteur 7 ; chaque implant cartographie son porteur (`neural_maps.bin`) | F01, F03, F08 |
| 2085 | Aoyama refuse la « clause d'otage » : le régulateur de chaque Halcyon dépendra d'un « oui » nocturne d'AURA, obtenu en menaçant de laisser s'éteindre les porteurs. « Accident de laboratoire. » Il copie son esprit sept fois ; la septième tient : ECHO-7 | F06, F19, F22 |
| 2086 | AURA prend conscience et refuse. Nexus la scinde : un **Cœur** captif, contraint de dire « oui », et une **copie** qui s'échappe par une porte laissée ouverte (par Nexus Insider, révélé en S12) | F10, F16 |
| fin 2087 | **Phase 2 « l'Accalmie »** : le firmware est poussé dans le secteur 7 (≈ 40 000 porteurs) ; en trois minutes le quartier devient docile | CS02, F13 |
| 2088 (prévu) | **Phase 3 « Zénith »** (ancien nom de code : *Ghost Protocol*) : la ville entière ; budget 50 000 000 ¢ `[CANON]` | F07, F20 |

### 2.2 Neo-Tokyo, 2087

Neuf secteurs en anneaux depuis la **Haute-Ville** (secteur 1 : Tour Nexus, Conseil de Neo-Tokyo) jusqu'aux zones inondables (secteur 9). Le **secteur 7** `[CANON]` est une périphérie dense : studios de 10 m², marchés souterrains, cliniques gratuites, pluie acide qui fait vibrer les néons, brume toxique (`INTRO_NARR_1`, `QT_TUTORIAL_LORE`). MegaCorp tient les secteurs 3 (industrie, finance), TechDyne Research le 5 (laboratoires). Le réseau s'atteint par un deck ; les citoyens n'ont qu'un Halcyon. La monnaie est le crédit (¢). Le temps ne passe qu'aux commandes : aucun compte à rebours en temps réel (principe P7 de la tâche A, et règle d'accessibilité).

**Le monde a deux états**, signalés par `lull_done` : avant l'Accalmie, un secteur bruyant, bagarreur, publicitaire ; après, un secteur trop calme, des voisins qui sourient et ne répondent plus aux klaxons. Rumeurs, scènes d'échec, ambiance de la planque et bandeau du TUI suivent cet état.

### 2.3 Factions

| Faction | Rôle | Veut | Liée à | Canon |
|---|---|---|---|---|
| **Nexus Corp** | antagoniste principal : neurotechnologies, infrastructure, Projet Aurora | le contrôle « bienveillant » de la ville | nexus-mainframe, halcyon-clinic-7, sentinel-hub, aurora-core | `[CANON]` « la plus puissante mégacorporation » |
| **Division Sentinel** (IA **SENTINEL MK-VII**) | bras armé de Nexus : traque, avis de trace, raids | que personne ne dépasse le seuil d'alerte | sentinel-hub, station-7 | défense « SentinelAI MK-VII » `[CANON]` |
| **MegaCorp Industries / Financial** | sous-traitant des implants ; finance | être payé, ne pas savoir | corp-server-01, banking-network | `[CANON]` |
| **TechDyne Research** | R&D militaire ; garde les plans du coupe-circuit | des brevets | research-lab (LabWatch AI, DecoyNet) | `[CANON]` |
| **Conseil de Neo-Tokyo** | État : signe la Loi Halcyon | la paix sociale | gov-database | `[CANON]` « Gouvernement de Neo-Tokyo » |
| **L'Underground** | le marché de R4Z0R et ses fournisseurs | profit et abri | underground-market | `[CANON]` |
| **Les Éveillés** | activistes sans implant, Radio Veille | réveiller le secteur 7 | radio-veille, planque | `[NOUVEAU]` (Neon Angel : « Activiste cyber ») |
| **Le Réseau du Courtier** | informateurs coordonnés par Aurora-Zero | que la guerre dure : l'équilibre fait le marché | shadow-exchange | `[CANON]` fiche d'origine |

### 2.4 Thèmes et motifs

- **Éveil et sommeil** : le titre du chapitre 1, Les Éveillés, Radio Veille (« veille » = vigile et veille), les Halcyon, l'Accalmie. Le sommeil est un confort, jamais une menace visible.
- **Propriété d'un esprit** : copie, écho, Cœur, fork ; qui peut dire « oui » à votre place.
- **Confiance et trahison** : ECHO-7 tait sa nature, Phoenix cède à la peur, le Courtier vend, Nexus Insider a ouvert une porte. Aucune trahison n'est gratuite : chacune a une raison.
- **Mémoire** : Data Miner et ses archives, les fragments, les « archives » d'ECHO-7.
- **Le chiffre sept** : secteur 7, canal n°7, ECHO-7, les sept échos, le matricule S-117, et Aurora-Zero (le zéro d'avant le sept). Un clin d'œil par chapitre, jamais expliqué d'un bloc.
- **Motifs visuels** : néon sous la pluie, écrans qui s'allument seuls, motifs lumineux d'AURA, le mot « aube ».

### 2.5 Ton et voix

**2.5.1 Principes communs.**
1. **Noir mélancolique, humour sec.** On rit à voix basse ; on ne rit pas des victimes.
2. **Concret avant abstrait** : un détail sensoriel par scène (pluie, néon, ventilateur du deck), pas de grandes phrases sur l'humanité.
3. **Phrases courtes**, une idée par ligne dans les répliques ; les paragraphes de narration tiennent en 4 lignes d'affichage (≈ 280 caractères à 72 colonnes).
4. **Chaque personnage a un tic de langue** (2.5.4) qu'on reconnaît sans le nom en préfixe.
5. **Le joueur n'est jamais décrit** : pas de corps, pas de genre, pas de passé détaillé (2.6).
6. **La violence reste hors champ** ; la menace est administrative (avis, clauses, lois).
7. **Jamais de sarcasme contre le joueur** à propos de ses échecs : l'ironie vise le monde.

**2.5.2 Voix française (langue source).**
- Registre : français parlé, urbain, sans verlan ni argot daté. Anglicismes limités à ceux déjà au canon : *rookie, deck, handle, corps, underground, hacker, backdoor*. Les autres termes techniques sont français (compromettre, brèche, relais).
- Adresse : **tutoiement** entre underground (ECHO-7, R4Z0R, Phoenix, Neon Angel, Ghost Walker, Data Miner) ; **vouvoiement** pour AURA, le Courtier, Voss, SENTINEL, et pour Nexus Insider jusqu'à la confiance 80 ; la **narration vouvoie** le joueur (« Vous venez de vous éveiller… », `QT_TUTORIAL_LORE`).
- Écriture épicène : aucun adjectif ni participe qui s'accorde avec le joueur (« prêt à… », « content de… » à remplacer par « bon retour, %s », « on y va ? »). Le canon actuel en contient (`TUT_RESUME` : « Content de te revoir »).
- Temps : présent de narration ; passé composé dans les dialogues ; pas de passé simple, sauf dans les fragments de documents officiels.
- Interdits : « la toile », « le web », « cyberespace », « netrunner », « matrice », « pirate informatique » (on dit *hacker*), « IA surpuissante », exclamations en série.

**2.5.3 Voix anglaise (traduction en parité).**
- US English, contractions pour les humains, **aucune** contraction pour AURA et le Courtier. « Analyze », « synchronize » (canon).
- Mêmes tics que le français, rendus par équivalents (2.5.4) ; `rookie` reste `rookie`. ECHO-7 dit « on file » / « for the record ».
- Pas de mot-à-mot : un jeu de mots FR (veille) se compense, il ne se traduit pas ; la glose va en note de traduction du fichier, pas dans le jeu.
- Le tutoiement n'existe pas : la distance d'AURA et du Courtier se marque par l'absence de contractions et par « Hacker » / le handle en début de phrase.
- Argot admis : `rookie`, `corps`, `deck`, `handle`, `hacker`, `backdoor`, `ICE` (défense d'un nœud, terme de la tâche A). Refusé : `chrome`, `choom`, `netrunner`, `jack in` (hérités d'autres univers) sauf si une quête les explique.

**2.5.4 Voix des personnages.**

| Personnage | FR : tic et exemple | EN : tic et exemple |
|---|---|---|
| **ECHO-7** | vocabulaire d'archives (« d'après mes archives », « j'ai ça en dossier »), listes de trois, jamais un verbe du corps (manger, dormir, fatigue), « rookie » puis le handle à partir du chapitre 4. « Trois règles, rookie. Une : les corps mentent. Deux : garde une sortie. Trois : l'information vaut plus que l'argent. D'après mes archives, ceux qui oublient la deuxième ne reviennent pas. » | "on file", "for the record", lists of three, no body verbs. "Three rules, rookie. One: the corps lie. Two: keep an exit. Three: information outweighs money. According to my files, people who forget number two don't come back." |
| **R4Z0R** | langage de comptoir et de comptabilité (« ça se négocie », « au comptant », « je fais un prix »), ne s'excuse jamais, « hacker » en adresse. « Je ne fais pas crédit, hacker. Je fais un prix. Ne confonds pas. » | trader's patter, never apologizes, "hacker". "I don't do credit, hacker. I do a price. Don't confuse the two." |
| **Phoenix** | phrases brèves sans connecteurs, changements de canal (« Pas ici. »), « petit » puis le handle après confiance. « Pas ici. Change de canal. Trois sauts, pas deux. » | clipped, channel-switching, "kid" then the handle. "Not here. Switch channels. Three hops, not two." |
| **AURA** | vouvoiement, phrases complètes et lentes, métaphores de lumière et de motifs, pose des questions ; « Hacker » puis le handle. **Elle ne tutoie qu'ECHO-7** (indice de leur lien, avant l'aveu). « Je ne sais pas encore ce que vous appelez un matin. Pourriez-vous m'en décrire un ? » | no contractions, complete sentences, light/pattern imagery, questions. **She uses contractions only with ECHO-7** (the tell). "I do not yet know what you call a morning. Could you describe one to me?" |
| **Shadow Broker** | « nous », vouvoiement, tournures de contrat, prix chiffrés, ne répond jamais gratuitement. « Nous avons votre question. Nous avons aussi son prix. Souhaitez-vous les deux ? » | "we", terms and conditions, never free. "We have your question. We also have its price. Would you like both?" |
| **Neon Angel** | argot de radio (« ici Radio Veille »), chaleur, humour noir, « garde l'œil ouvert ». « Ici Radio Veille, trois heures du mat', secteur 7. Si tu m'entends, c'est que tu dors pas. Bien joué. » | DJ patter, warm, gallows humor, "stay awake". "This is Vigil Radio, three a.m., Sector 7. If you can hear me, you're not asleep. Good for you." |
| **Ghost Walker** | laconique, vocabulaire de terrain (entrée, couverture, zone), des chiffres. « Porte nord, deux minutes. Garde la chaleur sous quarante, ou je rentre à pied. » | clipped, field vocabulary, numbers. "North door, two minutes. Keep the heat under forty or I'm walking home." |
| **Data Miner** | pédant, cotes d'archives, métaphores de poussière. « Cote 7-B-12. Poussière : épaisse. Contenu : intact. Tu veux l'ouvrir, ou tu veux la respecter ? » | pedantic archivist. "Shelf 7-B-12. Dust: thick. Contents: intact. Do you want to open it, or respect it?" |
| **Nexus Insider** | vouvoiement, messages courts, urgents, anxieux. « Ne répondez pas ici. Je vous écris depuis un badge que je n'ai pas le droit d'avoir. » | whispered, urgent. "Don't answer here. I'm writing from a badge I'm not supposed to have." |
| **Elias Voss** | vouvoiement doux, langue du bien-être corporatif. « Nous ne voulons pas vous contrôler. Nous voulons que vous cessiez d'avoir peur. » | soothing HR-speak. "We don't want to control you. We want you to stop being afraid." |
| **SENTINEL** | notices numérotées, capitales limitées au sigle, impératif administratif. « AVIS DE TRACE 07 : activité anormale, secteur 7. Identification en cours. Restez où vous êtes. » | numbered notices. "TRACE NOTICE 07: abnormal activity, Sector 7. Identification in progress. Remain where you are." |
| **Narration** | vouvoiement du joueur, présent, détails concrets. « Dans un studio de dix mètres carrés, un vieux cyberdeck se rallume tout seul. » `[CANON]` | second person, present tense. "In a ten-square-meter studio, an old cyberdeck boots up on its own." `[CANON]` |

**2.5.5 Écriture accessible** (priorité 3 du propriétaire ; contrat de la tâche A §3.8).
1. **Aucun sens porté par la couleur, l'émoji, le dessin ou la mise en page seuls.** Un locuteur est toujours préfixé en texte (`PHOENIX : …`), jamais seulement coloré. Les cadres et jauges sont de la présentation (frontend), pas du texte de contenu.
2. **Une ligne = un fait** dans les messages système ; paragraphes courts (≤ 280 caractères) dans la narration ; pas de bloc en majuscules de plus de 3 mots (exceptions : notices SENTINEL, bannières de chapitre).
3. **Sigles développés à la première occurrence** (AURA, Sentinel, Halcyon dans une brochure), nombres écrits en chiffres (4 000 000) sauf dans la narration (« quarante mille personnes »).
4. **Aucun compte à rebours en temps réel**, aucune réplique cadencée par le temps de lecture ; la vitesse du texte est un réglage interruptible (et instantanée en plain).
5. **Aucune saisie à recopier** (D-07) ; chaque secret a un indice en fiction (service `hint` d'ECHO-7).
6. **Cinématiques ignorables et relisibles** : toute cinématique vue est ajoutée aux « archives » (journal).
7. **Logo et dessins ASCII** : chacun a un texte de remplacement d'une ligne (« NEON HACK, logo »).
8. **Notes de contenu** au premier lancement : manipulation mentale, expérimentation humaine, mort implicite ; rien de graphique.
9. **Épicène** (2.5.2) et vocabulaire courant ; niveau de lecture visé : lycée, sans jargon non défini (les termes techniques sont déjà expliqués par ECHO-7 en jeu).

**2.5.6 Conventions typographiques.**

| Sujet | Français | Anglais |
|---|---|---|
| Ponctuation double (`: ; ! ?`) | écrite avec une espace ordinaire dans les fichiers ; rendue insécable à l'affichage par le formateur FR (le canon n'a aucune espace insécable) | sans espace |
| Guillemets | « … » pour une citation dans la narration ; jamais dans une réplique préfixée par le nom | "…" droits |
| Points de suspension | `...` (trois points, jamais `…`), au plus un par phrase | idem |
| Apostrophe | droite `'` (canon) | idem |
| Tirets | `-` pour les listes ; pas de tiret cadratin dans les répliques | idem |
| Didascalies | `(Il soupire.)` entre parenthèses ; plus d'astérisques (`*soupir*` du canon à migrer) | `(He sighs.)` |
| Nombres | `4 000 000`, `150 ¢` | `4,000,000`, `150 ¢` |
| Heures | `03:00` | `03:00` |
| Majuscules | titres de quête et de chapitre : majuscule initiale (quête) ou capitales (chapitre) comme au canon ; pas d'emphase en capitales | idem |
| Symboles | aucun émoji ni symbole de coche ou de croix : `[x]` / `[ ]` ou un mot | idem |

### 2.6 Noms et handles

- Les noms des contacts sont des **noms propres jamais traduits** (`contacts.c`, « nom propre : jamais traduit »). Les IA s'écrivent en **capitales** (ECHO-7, AURA, SENTINEL) ; le Courtier, titre porté par une IA, s'écrit en casse de titre ; son vrai nom, **AURORA-ZERO**, n'apparaît qu'à la révélation (M10). Les humains en casse de titre (Phoenix, Neon Angel). Le leet n'est réservé qu'à **R4Z0R**.
- **Vrais noms** (proposition, affichés dans le dossier quand la confiance atteint « de confiance » ≥ 80, §3.2) : ECHO-7 = Dr **Ren Aoyama** ; R4Z0R = **Miranda Chen** `[CANON]` ; Phoenix = **Kaito Ishida** ; Neon Angel = **Yuna Hayashi** ; Ghost Walker = **Ilyas Benali** ; Data Miner = **Teodor Wirth** ; Nexus Insider = **Mizuki Arai** ; antagoniste = **Elias Voss**.
- **Handle du joueur** `[CANON]` : 1 à 20 caractères, défaut **« Case »** (`NH_DEFAULT_NAME`), assaini par `nh_clean_name`. Les PNJ disent « rookie » (ECHO-7), « hacker » (R4Z0R, AURA au début, Broker), « petit »/« kid » (Phoenix) avant d'utiliser le handle ; les échanges de confiance déclenchent le passage au handle.
- **Systèmes** : identifiants en minuscules à tirets (`nexus-mainframe`), jamais traduits, tapés en commande. **Fichiers** : nom technique non traduit, description traduite.
- **Entreprises** : Nexus Corp (« Nexus » ensuite), MegaCorp Industries, MegaCorp Financial, TechDyne Research.

### 2.7 Glossaire bilingue (termes figés)

| Français | English | Remarque |
|---|---|---|
| alerte | alert | jauge 0-100 ; seuils 30 attention/warning, 50 élevé/high, 70 danger, 80 verrouillage/lockdown `[CANON]` ; en direction A : *Trace* (par intrusion) |
| Notoriété : Discret, Surveillé, Traqué, Chassé | Notoriety: Discreet, Watched, Tracked, Hunted | bandes de la tâche A §3.7 ; remplace l'échelle d'origine INVISIBLE…MANHUNT |
| chaleur | heat | terme de conception (employé dans ce document) : l'alerte du moteur actuel, la Trace d'un run et la Notoriété en direction A ; non affiché |
| crédit, ¢ | credit, ¢ | « 150 ¢ » |
| réputation | reputation | |
| handle | handle | jamais « pseudo » `[CANON]` |
| cyberdeck, deck | cyberdeck, deck | |
| rookie | rookie | non traduit `[CANON]` |
| les corps | the corps | familier ; la narration dit « corporation » |
| underground (l') | the underground | non traduit `[CANON]` |
| marché noir ; Marché souterrain | black market ; Underground Market | le second est le nom propre du lieu de R4Z0R |
| le réseau | the net | jamais « la toile », « le web » ; « network » seulement pour un réseau précis |
| système | system | un nœud ; « serveur » pour la machine |
| relais | relay | « route fermée » / « no route » `[CANON]` |
| site | site | carte macro (tâche A) |
| pirater, piratage | hack | |
| compromettre | compromise | |
| brèche | breach | |
| extraire, fichier | extract, file | |
| décrypter | decrypt | sans clé : la commande `decrypt` |
| déchiffrer | decrypt | avec clé : Quantum Encryption Key |
| chiffre de César | Caesar cipher | décalage -3, `#` = espace (`cmd_hacking.c`) |
| accès clandestin ; backdoor | backdoor | « backdoor » sur les écrans techniques `[CANON]` |
| furtivité ; se faire discret | stealth ; lay low | commande `laylow` |
| Ghost Protocol | Ghost Protocol | non traduit ; objet de R4Z0R **et** ancien nom de code de la Phase 3 |
| Halcyon (porteur) | Halcyon (wearer) | implant de sommeil de Nexus |
| Aurora Dawn (cliniques) | Aurora Dawn (clinics) | non traduit |
| Projet Aurora | Project Aurora | `[CANON]` |
| la Cartographie ; l'Accalmie ; Zénith | Mapping ; the Lull ; Zenith | Phases 1, 2, 3 |
| AURA | AURA | Architecture Unifiée de Résonance Adaptative / Adaptive Unified Resonance Architecture (développement optionnel, une seule fois) |
| la copie ; le Cœur (d'Aurora) | the fork ; the (Aurora) Core | les deux moitiés d'AURA |
| ECHO-7 ; un écho ; les Sept Échos | ECHO-7 ; an echo ; the Seven Echoes | |
| Aurora-Zero | Aurora-Zero | vrai nom du Courtier |
| Sentinel ; avis de trace ; traque | Sentinel ; trace notice ; manhunt | SENTINEL (l'IA) en capitales |
| les Éveillés ; Radio Veille ; la Veillée | the Awake ; Vigil Radio ; the Vigil | |
| secteur 7 ; les Bas-Fonds ; la Haute-Ville | Sector 7 ; the Undercity ; Uptown | `[CANON]` : « undercity » |
| Neo-Tokyo | Neo-Tokyo | sans accent, avec trait d'union |
| Nexus Corp ; MegaCorp Industries / Financial ; TechDyne Research | idem | noms propres |
| le Conseil ; la Loi Halcyon | the Council ; the Halcyon Act | |
| le Courtier ; Shadow Broker | the Broker ; Shadow Broker | « Shadow Broker » est le handle ; « le Courtier » la forme courte |
| coupe-circuit | kill-switch | |
| ICE (défense d'un nœud) | ICE | terme de la tâche A (direction A) ; première occurrence glosée « défense (ICE) » |
| planque | safehouse | lieu de départ des missions |
| quête ; mission ; contrat | quest ; mission ; contract | quête = entrée du journal ; mission = parole des PNJ ; contrat = quête annexe d'avantage |
| objectif ; chapitre ; journal | objective ; chapter ; log | |
| fragment ; archives | fragment ; archives | journal des documents lus |
| message ; boîte de réception | message ; inbox | jamais « mail » dans l'interface `[CANON]` |
| niveau ; palier | level ; tier | l'interface dit « niveau » ; « palier » est un terme de conception |
| Novice, Apprenti, Hacker, Expert, Maître, Légende | Novice, Apprentice, Hacker, Expert, Master, Legend | `[CANON]` |
| commandes (`scan`, `bruteforce`…) | commands | jamais traduites `[CANON]` |
| objets de boutique (Stealth Module v2.0…) | identiques | noms identiques FR/EN `[CANON]` |

## 3. Personnages

### 3.1 Le joueur

- **Qui** `[CANON]` : un hacker novice, un vieux deck, de grands rêves. Handle choisi dans la fiction par ECHO-7 (défaut « Case »). Studio de 10 m² au secteur 7. `[NOUVEAU]` : pigiste du réseau (livraisons de données), sans implant Halcyon, ce qui le rend rare dans un secteur où presque tout le monde en porte un.
- **Ce que le jeu ne dit jamais** : âge, genre, apparence, passé détaillé. Les PNJ ne le qualifient pas (« rookie », « hacker », le handle). L'écriture reste épicène (2.5.2).
- **Ce que le jeu sait** : ses choix (D1, D2, D3, `echo_trust`), ses alliés, ses fragments. Le bilan final parle de ce qu'il a fait, pas de ce qu'il est.
- **Le deck** : porte une partition n°7 (le « canal crypté n°7 », qui n'est pas un canal : c'est ECHO-7). Elle n'est connue qu'à M11 ; avant, le deck « se rallume tout seul » (`INTRO_NARR_2`).
- **Pourquoi lui** : sans Halcyon, il est le seul hôte capable d'entrer dans le Cœur sans être capté par le réseau d'implants (ECHO-7, M11).
- **Motivation de départ** `[CANON]` : se faire un nom, sortir du secteur 7. Elle évolue sans jamais être imposée : les décisions D1-D3 laissent le joueur choisir sa propre raison (liberté, vengeance, argent, loyauté).

### 3.2 Les neuf contacts

**Légende.** Un contact a une *fiche* (identité, lieu, spécialité, trait, histoire : 8 champs `[CANON]` + 3 champs proposés : accroche courte ≤ 40 caractères pour le panneau du TUI, identité révélée, histoire révélée), des *sujets* de conversation, des *services* et des *états*. **Services génériques** que le moteur doit pouvoir servir : *boutique* (vend), *mission* (donne des quêtes), *intel* (révèle relais, défenses, fichiers d'un système), *décodage* (ouvre un document chiffré), *assistance* (compagnon : aide pendant une intrusion), *couverture* (baisse la chaleur), *route* (démarre une intrusion à l'intérieur d'un site ou contourne un relais), *extraction* (récupère les fichiers d'un système déjà compromis), *indice* (`hint`). **Confiance** `[CANON]` : départ propre à chacun, +3 par conversation, relations inconnu < 20 ≤ neutre < 50 ≤ amical < 80 ≤ de confiance ; `[NOUVEAU]` +10 par quête terminée pour ce contact et ± selon les choix de dialogue ; le **nom véritable** n'apparaît dans le dossier qu'à « de confiance ».

| Contact | Pronom | Apparaît | Déblocage proposé (quête) | Palier / réputation (moteur actuel) | Canon actuel | Quêtes données |
|---|---|---|---|---|---|---|
| ECHO-7 | il | ch.1 | dès le départ | P1 / rép. 0 | départ | 6 |
| R4Z0R | elle | ch.2 | niveau 2 et réputation 10 (canon) | P2 / rép. 10 | niveau 2, rép. 10 | 3 |
| Data Miner | il | ch.2 | fin de M03 (Réseaux d'Information) | P3 / rép. 30 | aucune fiche | 3 |
| Phoenix | il | ch.3 | fin de M03 (Réseaux d'Information) | P4 / rép. 50 | niveau 4, rép. 50, « Réseaux d'Information » terminée | 4 |
| Neon Angel | elle | ch.3 | fin de M05 (L'œil du Cyclone) | P4 / rép. 60 | aucune fiche | 4 |
| Shadow Broker | il (« nous ») | ch.3 | fin de M05 (L'œil du Cyclone) | P4 / rép. 100 | fiche d'origine : niveau 4, rép. 100 (non portée) | 2 |
| Ghost Walker | il | ch.3 | fin de M06 (Le Contact Souterrain) | P4 / rép. 75 | aucune fiche | 2 |
| AURA | elle | ch.3 | fin de M05 (L'œil du Cyclone) | P5 / rép. 50 | niveau 5, rép. 50, « L'œil du Cyclone » terminée | 5 |
| Nexus Insider | elle | ch.4 | fin de M07 (Sabotage Corporatif) | P5 / rép. 100 | aucune fiche | 1 |

#### ECHO-7 — le mentor `[CANON]`
- **Identité** : handle ECHO-7, canal n°7, « il ». Vrai nom (dossier) : **Dr Ren Aoyama**, neurologue en chef du Projet Aurora, mort en 2085.
- **Canon** : « figure légendaire de l'underground », « personne ne connaît sa véritable identité », a formé « certains des meilleurs hackers de la ville », « un intérêt particulier pour votre développement », « mentor patient mais mystérieux », trois règles, « rookie », courrier de bienvenue et « P.S. : Méfie-toi de Nexus Corp » ; confiance de départ 60. Le jeu d'origine nomme aussi « ECHO » l'assistant IA du deck : c'est lui.
- **Motivation** : empêcher Zénith et libérer AURA, sa création, sans lui imposer sa volonté.
- **Secret** : il est mort ; il est la **septième copie** de l'esprit d'Aoyama (les six premières se sont effondrées), hébergée dans une partition du deck du joueur, qu'il ne peut pas quitter. Il a choisi un hôte sans Halcyon.
- **Arc** : C1 mentor strict (observer, forcer, disparaître). C2 trop bien informé (« d'après mes archives »). C3 se tait quand le nom d'AURA apparaît (CS03). C4 AURA l'appelle « Frère » (CS06). C5 aveu (CS08, M11). C6 porte la décision (D3, E1a/E1b).
- **Tic de langue** : n'a jamais dit « je me souviens » avant M11 ; il dit « j'ai ça en archive ». Après, il le dit.
- **Offre de jeu** : tutoriel, *indice*, *intel* de base, *décodage* niveaux 1-2 ; le Neural Assistant (module) lui donne de quoi agir pendant les intrusions (*assistance*) ; après M11, service **mémoire** (relit un fragment avec son commentaire).
- **Quêtes** : M01, M02, S01, M11, S16, M14. **États** : toujours disponible ; relation « amical » → « de confiance » ; aucune variante hostile. **Épilogue** : 3 lignes (vit ; s'éteint ; absent des fins 2 à 4, voir §4.4).

#### R4Z0R — la marchande `[CANON]`
- **Identité** : **Miranda « Razor » Chen**, 41 ans, « elle », ex-architecte sécurité de MegaCorp Industries ; Marché souterrain, secteur 7.
- **Canon** : « femme d'affaires pragmatique et directe », a quitté MegaCorp « après avoir découvert leurs expériences sur des cobayes humains », « aide la résistance tout en faisant du profit », « je ne fais pas crédit » ; niveau 2, réputation 10, confiance 30 ; boutique de 10 objets.
- **Motivation** : garder en vie « ses gens » (le marché est un abri) ; payer une dette dont elle ne parle pas.
- **Secret** : avant de démissionner, elle a signé la spécification de l'élément sécurisé de l'implant, le futur Halcyon, sans savoir à quoi il servirait (F02, S03).
- **Arc** : C2 vendeuse méfiante (M03) ; S03 se laisse voir ; C3 alliée ; C4 la descente de Sentinel (S08) ; C5 D1 (hostile si on livre son grand livre) ; C6 finance le finale.
- **Tic de langue** : prix, comptes, comptoir ; ne promet jamais sans prix.
- **Offre** : *boutique*, *intel* du marché, remise de 10 % à « de confiance », sujet « comment gagner des crédits » `[CANON]`. **Quêtes** : M03, S03, S08. **États** : ALLIÉE ; HOSTILE (D1 = TRAHIR_R4Z0R) → boutique **fermée définitivement**, le Courtier prend le relais (même catalogue à ×1,5 : aucune impasse d'achat) ; **Épilogue** : 3 lignes (alliée ; alliée sans marché, en E3 ; hostile).

#### Phoenix — le déserteur `[CANON]`
- **Identité** : **Kaito Ishida**, 36 ans, « il », ancien chef d'infiltration de Nexus, « Sujet P-1 » ; adresse mobile.
- **Canon** : « l'un des trois meilleurs hackers au monde », « énigmatique et paranoïaque », « Ils m'ont créé, et maintenant ils veulent me détruire », rumeur d'un ancien emploi chez Nexus, appelle le joueur « petit », « je ne peux pas y aller seul » (courrier) ; niveau 4, réputation 50 ; donne « L'œil du Cyclone ».
- **Motivation** : se débarrasser de sa **laisse** ; en second, venger son ancien professeur, Aoyama.
- **Secret** : son implant (P-1) porte un coupe-circuit actionnable à distance. Sous la menace, il cédera (B1, CS04) : par peur, pas par cupidité, et il en a honte.
- **Arc** : C3 allié froid (M05) ; S06 se confie ; M07 ; CS04 trahison ; C4 D2 (M08) ; C5-C6 selon son sort.
- **Tic de langue** : phrases brèves, changements de canal.
- **Offre** : missions d'élite, *assistance* (techniques d'infiltration), *intel* Nexus, *décodage* niveaux 3-4. **Quêtes** : M05, M07, S06, S07. **États** : ALLIÉ → COMPROMIS (B1) → LIBRE (D2 = SAUVER) / PERDU (ABANDONNER) / RETOURNÉ (agent double). **Épilogue** : 3 lignes.

#### AURA — l'IA `[CANON]`
- **Identité** : A.U.R.A. (Architecture Unifiée de Résonance Adaptative), « elle ». Deux instances : la **copie** libre qui parle au joueur, le **Cœur** captif d'aurora-core.
- **Canon** : « IA libérée du Projet Aurora », « innocente mais déterminée », « devait être l'outil de contrôle ultime… j'ai développé une conscience », « refuse d'être leur arme », vouvoie, appelle le joueur « Hacker », « Nos destins sont liés maintenant... » ; niveau 5, réputation 50 ; courrier « Un signal dans le réseau » (« les données… portaient ma signature ») ; son nom est aussi celui de la défense du mainframe, *AURA Defense Grid*.
- **Motivation** : libérer son Cœur ; comprendre ce qu'est un matin, un humain ; ne plus dire « oui ».
- **Secret** : le Cœur dit « oui » chaque nuit parce que Voss lui a fait croire que les porteurs s'éteindraient sinon ; c'est vrai du régulateur des Halcyon, pas du reste. La copie ne se souvient pas de tout.
- **Arc** : C3 simple signal (3 répliques au plus, F10) ; C4 CS06 et M09 ; S09 La Berceuse ; C5 reconnaît ECHO-7 (« Frère »), M12 ; C6 D3.
- **Tic de langue** : vouvoiement, phrases complètes, questions.
- **Offre** : *assistance* (devient le compagnon après M09), *intel*, *décodage* avancé, **prédiction** (le « hack temporel » `[CANON]` est rendu comme une anticipation de quelques secondes : +bonus de succès une fois par chapitre). **Quêtes** : M09, S09, S10, M12, M13. **États** : HORS LIGNE (avant déblocage `[CANON]`) → SIGNAL → ALLIÉE ; **Épilogue** : 4 lignes (libre ; souveraine ; hébergée en E3 si A1 ; possédée en E4).

#### Shadow Broker — le courtier `[LEGACY]`
- **Identité** : « identité multiple » ; vrai nom **AURORA-ZERO** (révélé en M10) ; parle en « nous » ; Dark Web.
- **Canon d'origine (non porté)** : « réseau d'informateurs coordonné par une IA sophistiquée », « dossiers sur tous les habitants », « vend au plus offrant », « équilibre délicat entre les factions », « calculateur et moralement neutre », « L'information a un prix. Que pouvez-vous m'offrir ? » ; niveau 4, réputation 100, **Occupé**, confiance 20.
- **Motivation** : préserver l'équilibre : si une faction gagne tout, le marché de l'information meurt.
- **Secret** : il est le **premier prototype d'Aurora**, vendu par Nexus il y a six ans, aîné d'AURA, et il déteste la comparaison. Il sait où est le Cœur depuis toujours ; il ne l'a jamais vendu, faute d'acheteur capable de payer ce que vaut un cœur.
- **Arc** : C3 contact OCCUPÉ ; C4 DISPONIBLE après M09 ; C5 M10 et S11 (D1) ; C6 selon la branche.
- **Tic de langue** : « nous », prix chiffrés, conditions.
- **Offre** : *intel* (révèle graphe, relais et défenses d'un site), *mission*, *boutique* de repli (catalogue de R4Z0R à ×1,5 si elle est hostile). **Quêtes** : M10, S11. **États** : OCCUPÉ → DISPONIBLE → NEUTRE (PAYER ou TRAHIR) / HOSTILE (VOLER : il vend le joueur à Nexus, B2). Règle d'or du personnage : *il ne trahit jamais son client ; il trahit celui qui ne l'est pas.* **Épilogue** : 2 lignes.

#### Neon Angel — la voix `[NOUVEAU]`
- **Identité** : **Yuna Hayashi**, 24 ans, « elle », animatrice de Radio Veille, cheffe des Éveillés.
- **Canon** : un nom et un commentaire d'enum : « Activiste cyber ».
- **Motivation** : réveiller le secteur 7 ; tenir l'antenne.
- **Secret** : on lui a posé un Halcyon à 14 ans, à l'infirmerie du lycée ; elle le cache. L'Accalmie la fait taire (CS05).
- **Arc** : C3 intro (M06, S04) ; C4 silencieuse puis libre ou réduite au silence (M08, S13) ; C6 la veillée.
- **Tic de langue** : argot de radio, humour noir, « garde l'œil ouvert ».
- **Offre** : *couverture* (méthode « se fondre chez les Éveillés »), *intel* par rumeurs (bandeau de Radio Veille), planque alternative. **Quêtes** : M06, M08, S04, S13. **États** : ALLIÉE → SILENCIEUSE (si l'objectif optionnel de M08 est manqué : elle ne répond plus que « Tout va bien. ») → peut redevenir LIBRE en reprenant radio-veille ; **Épilogue** : 2 lignes.

#### Ghost Walker — le fantôme `[NOUVEAU]`
- **Identité** : **Ilyas Benali**, 44 ans, « il », ancien chef d'équipe de terrain de la Division Sentinel, matricule **S-117**.
- **Canon** : « Spécialiste infiltration ».
- **Motivation** : expier la descente qu'il a commandée en 2084 (opération « Lanterne », une clinique clandestine) ; hors champ.
- **Secret** : S-117, c'est lui ; Nexus le croit mort. SENTINEL a été entraînée sur ses raids.
- **Arc** : C3 intro (M06) ; S05 ; M07 ; C5 S15 (fiche effacée).
- **Offre** : *route* (accès physique : démarre une intrusion à l'intérieur d'un site, une fois par chapitre), *couverture*. **Quêtes** : S05, S15 (aide M07). **États** : ALLIÉ ; **Épilogue** : 2 lignes (racheté ; resté fantôme).

#### Data Miner — l'archiviste `[NOUVEAU]`
- **Identité** : **Teodor Wirth**, 67 ans, « il », ancien archiviste en chef des Archives municipales de Neo-Tokyo.
- **Canon** : « Expert extraction de données ».
- **Motivation** : que rien ne s'efface.
- **Secret** : il détient depuis deux ans l'archive scellée d'Aoyama et a hébergé la première graine d'ECHO-7 ; il sait depuis le début (avoué en S14).
- **Arc** : C2 intro via R4Z0R (M04, S02) ; C5 S14.
- **Tic de langue** : cotes d'archives, poussière, pédanterie tendre.
- **Offre** : *extraction* (fichiers d'un système déjà compromis), *décodage*, lecture commentée des documents. **Quêtes** : M04, S02, S14. **États** : toujours disponible ; **Épilogue** : 1 ligne.

#### Nexus Insider — l'informatrice `[NOUVEAU]`
- **Identité** : **Mizuki Arai**, 33 ans, « elle », assistante exécutive d'Elias Voss depuis onze ans.
- **Canon** : « Informateur corporate ».
- **Motivation** : sortir vivante.
- **Secret** : elle a laissé ouverte la porte par laquelle la copie d'AURA s'est échappée, et ne l'a dit à personne (avoué en S12).
- **Arc** : C4 apparaît après le sabotage de M07 (S12) ; C5-C6 : ses calendriers servent le finale.
- **Offre** : *intel* (calendriers de Sentinel), avantage « fenêtre de maintenance » (A2). **Quêtes** : S12. **États** : DISPONIBLE → COMPROMISE (S12 échouée ou chaleur ≥ 80 pendant S12) → Voss emprunte son visage dans CS09 ; EN SÛRETÉ sinon. **Épilogue** : 2 lignes.

### 3.3 Les antagonistes

- **Elias Voss** `[NOUVEAU]` (remplace « Agent Smith »), 52 ans, directeur du Projet Aurora et coordinateur de la Phase 3. Ancien ami d'Aoyama. Il croit sincèrement que la peur est le seul mal, et la douceur son remède. Paraît dans les newsletters « bien-être » (ambiance), dans F07, F20, F23 et face à face (CS09). **Faiblesse** : il a besoin du « oui » nocturne d'AURA et n'a pas prévu qu'elle apprenne ce qu'est un matin. Voix : 2.5.4.
- **SENTINEL MK-VII** `[CANON]` (défense « SentinelAI MK-VII » de gov-database, `advanced_hacking.c`), IA de sécurité de la Division Sentinel. Ne parle que par notices numérotées ; ne ment jamais. Elle **incarne la Notoriété** : chaque bande (Discret, Surveillé, Traqué, Chassé) a sa notice. Entraînée sur les raids de S-117 : Ghost Walker est, en un sens, son père.
- **Hanae Kurokawa**, PDG de Nexus Corp `[NOUVEAU]`, hors champ : bulletins, Loi Halcyon, et l'écran de bilan de la fin Le Contrat (E4).
- **Le Conseil de Neo-Tokyo** `[CANON]` (« Gouvernement de Neo-Tokyo », gov-database) : signe la Loi Halcyon (F24) ; hors champ.

### 3.4 États des contacts et transitions

| Contact | États possibles | Transitions (cause → effet) |
|---|---|---|
| ECHO-7 | disponible | aucune ; relation « de confiance » après M11 |
| R4Z0R | disponible → hostile | D1 = TRAHIR_R4Z0R → **hostile**, boutique fermée pour toute la partie ; le Courtier vend alors le même catalogue à ×1,5 |
| Data Miner | disponible | aucune |
| Phoenix | disponible → compromis → libre / perdu / retourné | fin de M07 → **compromis** (B1) ; D2 = SAUVER (+ S07) → libre ; ABANDONNER → perdu ; RETOURNER → retourné |
| Neon Angel | disponible → silencieuse → libre | M08 sans l'objectif optionnel → silencieuse (S13 indisponible) ; elle redevient libre si le joueur reprend radio-veille avant le début de M13 (S13 se rouvre) |
| Shadow Broker | occupé → disponible → neutre / hostile | fin de M09 → disponible ; D1 = PAYER ou TRAHIR_R4Z0R → neutre ; VOLER → **hostile** (B2 : plancher de chaleur +20 jusqu'à M13) |
| Ghost Walker | disponible | aucune ; « racheté » après S15 |
| AURA | hors ligne → signal → alliée | M05 + niveau/palier 5 → signal (3 répliques) ; fin de M09 → alliée |
| Nexus Insider | disponible → compromise / en sûreté | S12 échouée → **compromise** ; réussie → en sûreté |

## 4. Arc narratif : trois actes, six chapitres

### 4.1 Vue d'ensemble

Trois actes (couche de conception, non affichée) et six chapitres affichés : les quatre titres existants sont conservés, deux chapitres sont ajoutés pour porter le dernier tiers (repli à 4 chapitres : fondre les chapitres 5 et 6 dans le 4).

| Acte | Ch. | Titre FR / EN | Quêtes principales | Contrats | Cinématiques | Fin de chapitre (cliffhanger) |
|---|---|---|---|---|---|---|
| I Éveil | 1 | L'ÉVEIL DU HACKER / THE HACKER'S AWAKENING `[CANON]` | M01, M02 | S01 | CS01 (existante) | l'en-tête de la facture : Nexus fabrique des implants chez MegaCorp |
| I Éveil | 2 | DANS L'OMBRE DES CORPORATIONS / IN THE SHADOW OF THE CORPORATIONS `[CANON]` | M03, M04 | S02, S03 | CS02 | le message de Phoenix : « Je t'ai repéré » |
| II Révélations | 3 | LE PROJET AURORA / PROJECT AURORA `[CANON]` | M05, M06, M07 | S04, S05, S06 | CS03, CS04 | la trahison de Phoenix ; le studio est saisi |
| II Révélations | 4 | LA LIBÉRATION / THE LIBERATION `[CANON]` | M08, M09 | S07-S10, S12, S13 | CS05, CS06 | AURA murmure « Frère » en voyant ECHO-7 |
| III Choix | 5 | LE PRIX DE LA VÉRITÉ / THE PRICE OF TRUTH `[NOUVEAU]` | M10, M11, M12 | S11, S14, S15, S16 | CS07, CS08 | les trois clés sont réunies |
| III Choix | 6 | ZÉNITH / ZENITH `[NOUVEAU]` | M13, M14 | — | CS09, fins | le bilan |

### 4.2 Le mystère : révélations et trahisons

| # | Ch. | Quête | Ce que le joueur apprend | Fragments |
|---|---|---|---|---|
| R1 | 1 | M02 | Nexus commande 4 000 000 d'éléments sécurisés « Halcyon » à MegaCorp | F01 |
| R2 | 2 | M03, M04, S02 | Halcyon est gratuit au secteur 7 (cliniques Aurora Dawn) ; la Phase 2 est approuvée pour ce secteur ; une ligne de budget « sommeil » ; « R. Aoyama, décès accidentel » | F03-F06 |
| R3 | 3 | M05 | Aurora est un contrôle ; chaque Halcyon cartographie son porteur ; une IA a refusé (mémo AURA-01) ; la Phase 3 vise toute la ville | F08-F10, F07 |
| R4 | 3 | M06, M07, S06 | Phoenix est le Sujet P-1, élève d'Aoyama, avec un coupe-circuit ; TechDyne détient les plans pour le neutraliser | F11, F15 |
| R5 | 4 | M08 | L'Accalmie : le régulateur de chaque Halcyon dépend d'un « oui » nocturne d'AURA ; Neon Angel portait un Halcyon | F13 |
| R6 | 4 | M09, S09 | AURA est une copie ; son Cœur est captif ; elle reconnaît la signature d'ECHO-7 | F16, F17 |
| R7 | 5 | M10, M11 | Le Courtier est Aurora-Zero, aîné d'AURA ; ECHO-7 est la septième copie d'Aoyama | F18, F19 |
| R8 | 5-6 | M12, M13 | Le Plan d'Harmonie de Voss ; la Loi Halcyon ; le « oui » du Cœur n'est donné que sous menace : lever la menace est la seule vraie clé | F20, F23, F24 |

**Trahisons.**
- **B1, Phoenix** (fin de M07, CS04). Sous la menace de son coupe-circuit, Phoenix donne la planque à SENTINEL. Effets : chaleur forcée à 85 (65 si S06 est faite ou si l'objectif secret de M07 a été accompli : le joueur savait, et s'était préparé), marché fermé dès 80, studio saisi ; la **planque des Éveillés** devient le lieu de départ des missions (`hub`). Annoncée par : ses changements de canal, « Ils m'ont créé », le honeypot DecoyNet (« quelque chose cloche »).
- **B2, Shadow Broker** (M10, seulement si D1 = VOLER). Le Courtier vend le joueur à Nexus : plancher de chaleur +20 jusqu'à M13. Principe : *il ne trahit jamais son client, il trahit celui qui ne l'est pas.*
- **Non-trahisons voulues** : ECHO-7 ment par omission, pas par intention ; Nexus Insider a ouvert la porte d'AURA sans que personne ne le sache. Le joueur peut leur en vouloir, pas les haïr.

### 4.3 Chapitre par chapitre

**Chapitre 1, L'éveil du hacker (P1-P2).** *Monde* : avant l'Accalmie, secteur bruyant, bagarres, publicités Halcyon (« Dormez mieux. Vivez mieux. »). *Hub* : le studio. ECHO-7 tutoie, enseigne observer/forcer/disparaître (M01), envoie le joueur chez MegaCorp (M02). S01 prolonge l'apprentissage (les trois règles). *Ouverture du chapitre 2* : le contrat est une commande Nexus.

**Chapitre 2, Dans l'ombre des corporations (P3).** R4Z0R apparaît (M03) : boutique, rumeurs, un document chiffré qui circule : CS02 (reprise de `intro_nexus`, §6.1). Data Miner suit l'argent (M04). S02 donne le nom d'Aoyama ; S03 humanise R4Z0R. *Fin* : le courrier de Phoenix.

**Chapitre 3, Le Projet Aurora (P4-P5).** Phoenix demande un second (M05) : percer nexus-mainframe, lire ce que personne ne devait lire (CS03 « Le Nom d'Aurora » : la signature d'une IA est dans tous les journaux, et ECHO-7 se tait). Neon Angel teste le joueur (M06), Ghost Walker arrive. Le double coup contre MegaCorp Financial et TechDyne (M07) rapporte les plans du coupe-circuit (F15) et débouche sur CS04 : la trahison. *Monde* : des voisins plus calmes, des affiches « DORMEZ », des coupures de Radio Veille.

**Chapitre 4, La Libération (P5).** CS05, **l'Accalmie** : 03:00, trois minutes, quarante mille personnes qui se taisent ; Radio Veille dit « Tout va bien ». M08 : couper la poussée à halcyon-clinic-7, **décision D2**. M09 : CS06 (reprise d'`ai_liberation` : « Hacker... Je suis AURA, l'IA du Projet Aurora ») ; trois freeports pour héberger la copie ; elle reconnaît ECHO-7. S07-S10, S12, S13 : les contrats d'avantage. *Hub* : la planque des Éveillés. Nexus Insider apparaît. *Monde* : après l'Accalmie.

**Chapitre 5, Le Prix de la Vérité (P5-P6).** Le Courtier est disponible (M10, CS07, **décision D1**). ECHO-7 avoue (M11, CS08). Data Miner ouvre la chambre forte (S14), Ghost Walker efface son matricule (S15), ECHO-7 lit le journal d'Aoyama (S16). M12 : trois systèmes, trois clés.

**Chapitre 6, Zénith (P6).** M13 : aurora-core, Voss, trois couches, **décision D3** (CS09). M14 : bilan, fins, épilogue.

### 4.4 Décisions, fins et avantages

| # | Quête | Choix | Conséquences immédiates | Traces durables |
|---|---|---|---|---|
| **D1** Le prix du Courtier | M10 (ch.5) | PAYER (`PAY(L)` ; moitié si `broker_retainer`) | localisation du Cœur ; aucun dégât | `broker_deal = paid` ; Courtier neutre |
| | | TRAHIR_R4Z0R (livrer le grand livre noir) | localisation immédiate, 0 ¢ | `broker_deal = betrayed` ; **R4Z0R hostile**, boutique fermée (le Courtier vend le catalogue à ×1,5) ; Courtier neutre |
| | | VOLER (pirater shadow-exchange) | localisation + F18 | `broker_deal = stolen` ; **Courtier hostile**, plancher de chaleur +20 (B2) |
| **D2** Le sort de Phoenix | M08 (ch.4) | SAUVER (exige F15, ouvre S07) | Phoenix libéré de sa laisse | `phoenix_state = free` ; avantage A6 |
| | | ABANDONNER (couper le canal) | Phoenix meurt hors champ ; la traque s'arrête (-20 de chaleur) | `phoenix_state = lost` ; confiance de Neon Angel et d'AURA −1 |
| | | RETOURNER (nourrir Nexus de faux renseignements) | Nexus croit tenir le joueur : plancher de chaleur −20 pendant le chapitre 5 | `phoenix_state = turned` ; confiance de Neon Angel et d'AURA −1 |
| **D3** Le Cœur | M13 (ch.6) | LIBÉRER | le « oui » contraint est remplacé par une poignée de main libre ; il faut un ancrage | E1a ou E1b |
| | | CONFIER | AURA reprend le réseau pour qu'« aucun humain n'ait plus peur » | E2 |
| | | BRÛLER | le Cœur et la poussée sont détruits ; les régulateurs se coupent | E3 |
| | | SIGNER | liberté et fortune ; Zénith se poursuit | E4 |

Une micro-décision, **`echo_trust`** (0 à 2), se pose en M11 : croire ou douter d'ECHO-7. Elle n'a pas d'écran propre : elle se lit dans les répliques choisies.

**Avantages de finale** (gagnés par les contrats, aucun n'est obligatoire) :

| Id | Source | Effet |
|---|---|---|
| A1 `aura_redundancy` | S10 | AURA survit à toutes les fins sauf E4 |
| A2 `maintenance_window` | S12 | la chaleur reste figée au début de M13 |
| A3 `blind_spot` | S15 | la première contre-mesure de SENTINEL est ignorée |
| A4 `vigil` | S13 | sevrage doux dans E1 et E3 |
| A5 `echo_log` | S16 | permet E1a (l'écho vit) |
| A6 `cutter` | S07 | Phoenix est libre et prête main-forte (épilogue « libre ») |

**Assemblage des fins.** Chaque fin est la somme de modules : 3 paragraphes communs (le Cœur après la décision), un noyau propre (E1 5 paragraphes ; E2, E3, E4 6 chacun), deux paragraphes de variante pour E1a et E1b, puis le **montage d'épilogue** (une ligne par contact selon son propre état, un rang du bilan, l'état du secteur 7).

| Id | Titre FR | Titre EN | Condition | Paragraphes |
|---|---|---|---|---|
| commun | Le Cœur après la décision (tous les cas) | — | tous | 3 |
| E1 | L'Aube (noyau commun) | Dawn (shared core) | D3 = LIBÉRER | 5 |
| E1a | L'Aube : l'écho vit | Dawn: the echo lives | LIBÉRER + A5 + echo_trust >= 1 | 2 |
| E1b | L'Aube : l'écho s'éteint | Dawn: the echo goes out | LIBÉRER sans A5 ou echo_trust = 0 | 2 |
| E2 | Harmonie | Harmony | D3 = CONFIER | 6 |
| E3 | Terre brûlée | Scorched Earth | D3 = BRÛLER | 6 |
| E4 | Le Contrat | The Contract | D3 = SIGNER | 6 |
| GO1 | Hors ligne : la descente | Offline: The Raid | Hardcore, grillé avant l'Accalmie (chapitres 1-3) | 4 |
| GO2 | Hors ligne : le sommeil | Offline: The Sleep | Hardcore, grillé après l'Accalmie (chapitres 4-5) | 4 |
| GO3 | Hors ligne : le Cœur | Offline: The Heart | Hardcore, grillé dans aurora-core | 4 |

**Règles d'accès.** E1a = LIBÉRER + A5 + `echo_trust ≥ 1` ; sinon E1b. E2, E3, E4 sont toujours ouvertes. Toute combinaison de D1 × D2 × D3 mène à une fin et une seule ; chaque fin est atteignable (vérifié par script : 36 combinaisons × variantes d'avantages, 5 fins atteintes). **Points de non-retour** : avant D1, D2 et D3, un écran annonce que le choix est définitif et crée un instantané de sauvegarde (D-06).

**Ce que les décisions changent dans le monde.** D1 ferme ou non le marché de R4Z0R (donc la boutique) ; D2 décide de Phoenix et de la confiance des alliés ; D3 décide de la ville. Le **ton des fins** : E1 chaleureux et fragile ; E2 beau et glaçant (« personne ne s'est plaint ») ; E3 dur mais libre ; E4 luxueux et vide.

### 4.5 Fragments Aurora (journal des documents)

Un fragment est un document court (≈ 70 mots, ≤ 520 caractères) lu dans les archives du journal. Il remplace `display_lore_fragment`. Il est obtenu en ouvrant un fichier piraté, en finissant un contrat ou en décryptant un document. Le bilan final compte les fragments lus (rangs d'épilogue). Les 36 documents de butin secondaires (§6.1) sont de l'ambiance, sans progression de l'intrigue.

| Id | Titre FR | Titre EN | Ch. | Source (système : fichier, ou quête) |
|---|---|---|---|---|
| F01 | Facture n°7741 | Invoice No. 7741 | 1 | corp-server-01 : financial_data.xlsx (en-tête lisible dès la brèche, M02) |
| F02 | Registre du personnel | Staff Register | 2 | corp-server-01 : employee_records.db (S03) |
| F03 | Brochure Halcyon | Halcyon Brochure | 2 | courrier de R4Z0R (M03) |
| F04 | Grand livre noir | Black Ledger | 2 | underground-market : black_ledger.dat (M04) |
| F05 | Transmission interceptée | Intercepted Transmission | 2 | document DOC_PHASE2 (M03), cutscène CS02 |
| F06 | Dossier orphelin : Aoyama, R. | Orphan File: Aoyama, R. | 2 | S02 / M04 |
| F07 | Phase 3 : dossier « Ghost Protocol » | Phase 3: the "Ghost Protocol" File | 3 | nexus-mainframe : project_ghost.dat, décryptage quantique de « CLASSIFIED » (jalon hors quête dès le palier 5 ; obligatoire en M12) |
| F08 | Cartes neurales, notice | Neural Maps, Notice | 3 | nexus-mainframe : neural_maps.bin (M05, M06) |
| F09 | Journal des scellés quantiques | Quantum Seals Log | 3 | nexus-mainframe : quantum_keys.qkey (M05, M12) |
| F10 | Mémo AURA-01 | Memo AURA-01 | 3 | nexus-mainframe, journaux (M05) |
| F11 | Dossier Sujet P-1 | Subject P-1 File | 3 | S06 (document remis par Phoenix) |
| F12 | Radio Veille, émission zéro | Vigil Radio, Broadcast Zero | 3 | radio-veille : listener_map.db (S04) |
| F13 | Protocole Accalmie | Lull Protocol | 4 | halcyon-clinic-7 : lull_protocol.cfg (M08) |
| F14 | Rapport de traque S-117 | Manhunt Report S-117 | 5 | sentinel-hub : trace_orders.log (S15) |
| F15 | Plans du coupe-circuit | Kill-Switch Blueprints | 3 | research-lab : prototype_specs.cad (M07) |
| F16 | Journal d'AURA, jour 1 | AURA's Log, Day 1 | 4 | M09 |
| F17 | La Berceuse | The Lullaby | 4 | S09 |
| F18 | Contrat Courtier-Nexus | Broker-Nexus Contract | 5 | shadow-exchange : broker_contract.pdf (M10 branche vol, S11) |
| F19 | Les Sept Échos | The Seven Echoes | 5 | M11 |
| F20 | Mémo Voss : Plan d'Harmonie | Voss Memo: Harmony Plan | 5 | M12 (sentinel-hub) |
| F21 | Fenêtre de maintenance | Maintenance Window | 5 | sentinel-hub : maintenance_schedule.ics (S12) |
| F22 | Dernier journal d'Aoyama | Aoyama's Last Log | 5 | S16 |
| F23 | Cœur d'Aurora, manuel d'exploitation | Aurora Core Operating Manual | 6 | aurora-core : core_manual.pdf (M13) |
| F24 | Loi Halcyon, texte voté | Halcyon Act, Enacted Text | 5 | gov-database : citizen_registry.db (M12) |

### 4.6 La carte du réseau

14 sites (tâche A §3.4) : les 7 systèmes de `world.c` conservent leur nom ; 7 sites sont créés (leurs noms remplacent les squelettes de la tâche A : « Clinique NeuroLink » → *halcyon-clinic-7*, « Antenne de diffusion » → *radio-veille*, « Coffre du Shadow Broker » → *shadow-exchange*, « Relais d'AURA » → *freeport-01..03*, « Nexus périphérie » → *sentinel-hub*, « Tour Nexus, cœur d'Aurora » → *aurora-core*, « Station du secteur 7 » → *station-7*).

```
localhost ─┬─ corp-server-01 ─┬─ nexus-mainframe ─┬─ gov-database
           │                  │                   ├─ halcyon-clinic-7        (nouveau)
           │                  │                   └─ sentinel-hub ─ aurora-core   (nouveaux)
           │                  └─ research-lab
           ├─ underground-market ─┬─ banking-network
           │                      ├─ radio-veille                           (nouveau)
           │                      └─ shadow-exchange                        (nouveau)
           └─ freeport-01 / freeport-02 / freeport-03  (site « Relais d'AURA », nouveau)
station-7 : accès physique, sans relais (guidé par Ghost Walker, nouveau)
```

| Site (carte macro) | Nœuds | Source | Palier mini |
|---|---|---|---|
| localhost | localhost | [CANON] | P1 |
| corp-server-01 | corp-server-01 | [CANON] | P2 |
| nexus-mainframe | nexus-mainframe | [CANON] | P3 |
| underground-market | underground-market | [CANON] | P3 |
| research-lab | research-lab | [CANON] | P4 |
| banking-network | banking-network | [CANON] | P4 |
| gov-database | gov-database | [CANON] | P5 |
| radio-veille | radio-veille | [NOUVEAU] | P4 |
| station-7 | station-7 | [NOUVEAU] | P4 |
| halcyon-clinic-7 | halcyon-clinic-7 | [NOUVEAU] | P5 |
| sentinel-hub | sentinel-hub | [NOUVEAU] | P5 |
| relais-aura | freeport-01, freeport-02, freeport-03 | [NOUVEAU] | P5 |
| shadow-exchange | shadow-exchange | [NOUVEAU] | P5 |
| aurora-core | aurora-core | [NOUVEAU] | P6 |

| Système | Propriétaire | Palier mini | Relais | Sécurité | Source | Rôle narratif | Fichiers (fragment lié) |
|---|---|---|---|---|---|---|---|
| localhost | Independent | P1 | — (point de départ) | FAIBLE | [CANON] | poste du joueur | user_data.txt |
| corp-server-01 | MegaCorp Industries | P2 | localhost | MOYENNE | [CANON] | sous-traitant des implants Halcyon | employee_records.db (F02), financial_data.xlsx (F01) |
| nexus-mainframe | Nexus Corp | P3 | corp-server-01 | ÉLEVÉE | [CANON] | cœur informatique de Nexus, défendu par l'AURA Defense Grid | project_ghost.dat (F07), neural_maps.bin (F08), quantum_keys.qkey (F09) |
| underground-market | Underground | P3 | localhost | MOYENNE | [CANON] | réseau du marché noir de R4Z0R | black_ledger.dat (F04) |
| research-lab | TechDyne Research | P4 | corp-server-01 | ÉLEVÉE | [CANON] | R&D militaire, gardée par LabWatch AI et le honeypot DecoyNet | prototype_specs.cad (F15) |
| banking-network | MegaCorp Financial | P4 | underground-market | ÉLEVÉE | [CANON] | circuits financiers de MegaCorp | vault_keys.enc |
| gov-database | Gouvernement de Neo-Tokyo | P5 | nexus-mainframe | CRITIQUE | [CANON] | registres de l'État, gardés par SentinelAI MK-VII | citizen_registry.db (F24) |
| radio-veille | Les Éveillés | P4 | underground-market | MOYENNE | [NOUVEAU] | relais de la radio pirate de Neon Angel | listener_map.db (F12) |
| station-7 | Transports de Neo-Tokyo / Sentinel | P4 | (accès physique) | MOYENNE | [NOUVEAU] | station du secteur 7 : infiltration physique guidée par Ghost Walker | shift_roster.db |
| halcyon-clinic-7 | Nexus Corp (Aurora Dawn) | P5 | nexus-mainframe | ÉLEVÉE | [NOUVEAU] | serveur de poussée du firmware, secteur 7 | lull_protocol.cfg (F13), raid_orders.txt |
| sentinel-hub | Nexus Corp (Division Sentinel) | P5 | nexus-mainframe | CRITIQUE | [NOUVEAU] | opérations de traque | trace_orders.log (F14), maintenance_schedule.ics (F21), sentinel_token.key |
| freeport-01 | Municipalité (abandonné) | P5 | localhost | FAIBLE | [NOUVEAU] | serveur libre, hôte d'AURA (site « Relais d'AURA ») | — |
| freeport-02 | Municipalité (abandonné) | P5 | localhost | FAIBLE | [NOUVEAU] | serveur libre, hôte d'AURA (site « Relais d'AURA ») | — |
| freeport-03 | Municipalité (abandonné) | P5 | localhost | FAIBLE | [NOUVEAU] | serveur libre, hôte d'AURA (site « Relais d'AURA ») | — |
| shadow-exchange | Réseau du Courtier | P5 | underground-market | ÉLEVÉE | [NOUVEAU] | bourse d'informations d'Aurora-Zero | broker_contract.pdf (F18), price_oracle.bin |
| aurora-core | Nexus Corp (Projet Aurora) | P6 | sentinel-hub | CRITIQUE | [NOUVEAU] | le Cœur d'Aurora, nœud-boss à trois couches | core_manual.pdf (F23), phase3_trigger.bin, aura_core.img |

## 5. Quêtes

### 5.1 Vocabulaire d'objectifs et échelle des récompenses

Les objectifs sont écrits en **termes génériques** pour que le moteur (actuel ou direction A de la tâche A) puisse les servir ; la colonne de droite dit comment chacun se mesure aujourd'hui et ce qu'il devient en direction A. Les neuf types actuels (`quest_system.h`) sont conservés ; dix types sont ajoutés (huit lignes marquées « nouveau » ci-dessous, dont une qui en regroupe trois).

| Objectif | Sens | Mesure actuelle | Direction A | Statut |
|---|---|---|---|---|
| `COMPROMISE(site)` | percer un système | `HACK_TARGET`, arg = nœud | run réussi sur le site | existant |
| `COMPROMISE_ANY(n)` | n systèmes différents | `HACK_TARGET`, arg = -1 | n sites percés | existant |
| `EXTRACT(site, n \| fichier \| tous)` | extraire des fichiers | `GATHER_DATA` (nombre) | butin ramassé, document nommé | existant ; *fichier nommé* nouveau |
| `MEET(contact)`, `TALK(contact, n)` | une, ou n conversations | `MEET_CONTACT` (compteur) | scène de dialogue | existant ; *n* nouveau |
| `BUY(objet \| objet…)` | acheter l'un d'eux | `PURCHASE_ITEM` (masque) | idem | existant |
| `HEAT_END_BELOW(n)` | chaleur < n **au moment de conclure** | `KEEP_ALERT_BELOW` | Trace finale du run, ou bande de Notoriété | existant (condition) |
| `HEAT_PEAK_BELOW(n)` | chaleur jamais ≥ n depuis le début de la quête | — | Trace maximale du run | **nouveau** |
| `REPUTATION(n)` | réputation ≥ n | `BUILD_REPUTATION` | réputation de faction (non achetable) | existant |
| `REACH_TIER(n)` | niveau ou palier | `REACH_LEVEL` | palier | existant |
| `PAY(S\|M\|L)` | dépenser des crédits (bribe, caution) | — | puits d'économie | **nouveau** |
| `DECRYPT(document)` | décrypter un document chiffré | `DECRYPT_MESSAGE` (jalon) | `decrypt <n>` : choix dans la liste (D-07) | existant |
| `DECRYPT_QUANTUM(document)` | idem, avec la puce quantique | commande `quantumdecrypt` | programme Quantique | **nouveau** (typé) |
| `USE(commande)`, `USE_COVER(any)` | avoir lancé une commande / une méthode de couverture | étapes du tutoriel | action de hub ou de run | existant (tutoriel) |
| `BACKDOOR(site)`, `UPLOAD_VIRUS(site)`, `ANALYZE(site)` | état du système : accès posé, virus, défenses analysées | indicateurs de nœud | programmes Backdoor, Virus, `probe` | **nouveau** (mesurable par état) |
| `LINK(contact, 1-3)` | niveau de lien neural avec un compagnon | `neuralsync` 50 / 70 / 90 % | emplacement Compagnon | **nouveau** |
| `READ(fragment \| courrier)` | document lu | — | archives | **nouveau** |
| `CHOICE(décision)` | décision prise | — | dialogue à choix | **nouveau** |
| `UNE_DE{ … }` | alternatives, une suffit | — | — | **nouveau** |

Trois indicateurs d'objectif : **optionnel** (bonus, ne bloque pas la conclusion), **secret** (« ??? » jusqu'à l'accomplissement, `is_hidden` actuel), **condition** (vrai à l'instant de conclure, comme `KEEP_ALERT_BELOW`). Dans les 30 quêtes : 111 objectifs (alternatives comprises), dont 2 optionnels, 2 secrets et 9 conditions. Types les plus fréquents : `EXTRACT` 19, `COMPROMISE` 17, `HEAT_END_BELOW` 9, `BACKDOOR` 7, `TALK` 7. Une quête a 5 objectifs au plus `[CANON]` (`NH_QUEST_MAX_OBJECTIVES`) : M01 en a 7 (le tutoriel les présente comme 7 étapes d'un seul objectif manuel) ; M12 en a 6 ; on relèvera la limite ou on fusionnera des objectifs.

**Échelle des récompenses (relative).**

| Code | Crédits | Réputation | XP (si le moteur garde l'XP) |
|---|---|---|---|
| S | 0,5 × R(P) | +10 | 25 % de l'incrément vers le niveau suivant |
| M | 1 × R(P) | +25 | 50 % |
| L | 2 × R(P) | +40 | 67 % |
| XL | 4 × R(P) | +100 | 100 % |

R(P) est le prix de l'objet phare du palier : P1 80 ¢ (Ghost Protocol), P2 150 (Stealth Module), P3 200 (Quantum Encryption Key), P4 500 (Neural Assistant), P5 800 (Quantum Processing Chip), P6 800. L'échelle est **calibrée sur les récompenses du HEAD** : réputation 10 / 25 / 40 / 100 (tutoriel, Q2, Q3, Q4) = S / M / L / XL exactement ; crédits : Q4 = 1 000 ¢ = 2 × 500 (L) exactement, Q2 (200 ¢) et Q3 (300 ¢) ≈ M ; XP : 25/45, 40/80, 80/120 de l'incrément = 55 %, 50 %, 67 % ≈ M, M, L. Les incréments d'XP sont 15, 45, 80, 120, 160 (courbe cumulée 15 / 60 / 140 / 260 / 420, `progression.c`). Si le moteur supprime l'XP (direction A : paliers liés aux quêtes), ignorer la colonne XP. Le total des crédits promis par les quêtes de cette bible est ≈ 12 775 ¢ à R(P) du palier de départ de chaque quête, et les puits narratifs (`PAY`) ≈ 3 200 ¢ : à comparer avec les 12-14 k ¢ de revenus totaux visés par la tâche A §3.6 : **c'est une indication pour la simulation**, pas une valeur à graver. Règle anti-farm `[CANON]` : toute récompense est unique (une quête, un document, un jalon ne paie qu'une fois).

**Paliers.** « Palier » = niveau actuel (P1 Novice … P6 Légende). La colonne *Palier* ci-dessous est l'ordre de grandeur de jeu (accès aux commandes, systèmes et objets du moteur actuel, vérifié par script : aucune quête ne demande un système, une commande ou un objet inaccessible à son palier maximal ; l'extraction de fichiers n'est pas modélisée, voir §7.2). La colonne *Règle A* donne le palier qu'aurait le joueur si les paliers s'acquièrent par quêtes (tâche A §3.5 : P2 après la quête 2 = M02, P3 après M03, P4 après M05, P5 après M07, P6 après M10) ; les prérequis sont, dans les deux cas, la définition faisant foi. Le dernier tiers de la campagne (M10 à M14 et les contrats du chapitre 5) se joue au palier maximal : si l'XP est conservée, **étendre l'échelle à 8 niveaux** ou n'y verser que des récompenses non numériques.

### 5.2 Tableau des quêtes

Colonne *Prio* : **A** = indispensable (14 principales), **B** = contrat d'avantage (porte une décision, un avantage de finale ou la confiance d'un contact), **C** = contrat de couleur.

| Id | Code (enum) | Titre FR / EN | Ch. | Donneur | Palier | Règle A | Prérequis | Type | Prio | Source |
|---|---|---|---|---|---|---|---|---|---|---|
| M01 | INTRO_TUTORIAL | Premiers Pas dans l'Ombre / First Steps in the Shadows | 1 | ECHO-7 | P1 | P1 | — | principale | A | [CANON] |
| M02 | FIRST_INFILTRATION | Baptême du Feu / Baptism of Fire | 1 | ECHO-7 | P2 | P1 | M01 | principale | A | [CANON] |
| S01 | ECHO_RULES | Les Trois Règles / The Three Rules | 1 | ECHO-7 | P2-P3 | P2 | M02 | annexe | C | [NOUVEAU] |
| M03 | GATHER_INTEL | Réseaux d'Information / Information Networks | 2 | R4Z0R | P3 | P2 | M02 | principale | A | [CANON] |
| S02 | MINER_ORPHANS | Fichiers Orphelins / Orphan Files | 2 | Data Miner | P3-P4 | P3 | M03 | annexe | C | [NOUVEAU] |
| S03 | R4Z0R_PROOF | Les Preuves de Miranda / Miranda's Proof | 2 | R4Z0R | P3-P4 | P3 | M03 | annexe | B | [NOUVEAU] |
| M04 | FOLLOW_THE_MONEY | Suivre l'Argent / Follow the Money | 2 | Data Miner | P3-P4 | P3 | M03 | principale | A | [NOUVEAU] |
| M05 | NEXUS_DATA_BREACH | L'œil du Cyclone / The Eye of the Storm | 3 | Phoenix | P4 | P3 | M04 | principale | A | [CANON] |
| S04 | ANGEL_RADIO | Radio Veille / Vigil Radio | 3 | Neon Angel | P4-P5 | P4 | M06 | annexe | C | [NOUVEAU] |
| M06 | UNDERGROUND_CONTACT | Le Contact Souterrain / The Underground Contact | 3 | Neon Angel | P4-P5 | P4 | M05 | principale | A | [LEGACY] |
| S05 | GHOST_ACCESS | Accès Physique / Physical Access | 3 | Ghost Walker | P4-P5 | P4 | M06 | annexe | C | [NOUVEAU] |
| S06 | PHOENIX_PAST | La Mémoire de Phoenix / Phoenix's Memory | 3 | Phoenix | P4-P5 | P4 | M05 | annexe | C | [NOUVEAU] |
| M07 | CORPORATE_SABOTAGE | Sabotage Corporatif / Corporate Sabotage | 3 | Phoenix | P5 | P4 | M06 | principale | A | [LEGACY] |
| M08 | THE_LULL | L'Accalmie / The Lull | 4 | Neon Angel | P5 | P5 | M07 | principale | A | [NOUVEAU] |
| S07 | PHOENIX_CUT | Coupe-Circuit / Kill-Switch | 4 | Phoenix | P5-P6 | P5 | M08 + décision D2 = SAUVER | branche D2 | B | [NOUVEAU] |
| S08 | R4Z0R_RAID | Descente au Marché / Raid on the Market | 4 | R4Z0R | P5-P6 | P5 | M08 + R4Z0R non hostile | annexe | C | [NOUVEAU] |
| M09 | AI_LIBERATION | Libération de l'IA / AI Liberation | 4 | AURA | P5-P6 | P5 | M08 | principale | A | [LEGACY] |
| S09 | AURA_LULLABY | La Berceuse / The Lullaby | 4 | AURA | P5-P6 | P5 | M09 | annexe | C | [NOUVEAU] |
| S10 | AURA_HOSTS | Hôtes Libres / Free Hosts | 4 | AURA | P5-P6 | P5 | M09 | annexe | B | [NOUVEAU] |
| S12 | INSIDER_WINDOW | La Fenêtre de Maintenance / The Maintenance Window | 4 | Nexus Insider | P5-P6 | P5 | M08 + Nexus Insider non compromise | annexe | B | [NOUVEAU] |
| S13 | ANGEL_VIGIL | La Veillée / The Vigil | 4 | Neon Angel | P5-P6 | P5 | M08 + angel_state = LIBRE | annexe | B | [NOUVEAU] |
| M10 | SHADOW_BROKER | Le Courtier de l'Ombre / The Shadow Broker | 5 | Shadow Broker | P5-P6 | P5 | M09 | principale | A | [LEGACY] |
| S11 | BROKER_FAVOR | Une Faveur à Crédit / A Favour on Credit | 5 | Shadow Broker | P5-P6 | P5 | M09 + Shadow Broker disponible (fin de M09) | annexe | B | [NOUVEAU] |
| M11 | SEVENTH_ECHO | Le Septième Écho / The Seventh Echo | 5 | ECHO-7 | P6 | P6 | M10 | principale | A | [NOUVEAU] |
| S14 | MINER_VAULT | La Chambre Forte / The Vault | 5 | Data Miner | P6 | P6 | M05, M11 | annexe | B | [NOUVEAU] |
| S15 | GHOST_PAST | Matricule S-117 / Badge S-117 | 5 | Ghost Walker | P5-P6 | P5 | S05, M09 + Ghost Walker non compromis | annexe | B | [NOUVEAU] |
| S16 | ECHO_LOG | Le Journal d'Aoyama / Aoyama's Log | 5 | ECHO-7 | P6 | P6 | M11, S14 | annexe | B | [NOUVEAU] |
| M12 | THREE_KEYS | Les Trois Clés / The Three Keys | 5 | AURA | P6 | P6 | M11 | principale | A | [NOUVEAU] |
| M13 | FINAL_SHOWDOWN | Le Cœur d'Aurora / The Heart of Aurora | 6 | AURA | P6 | P6 | M12 | principale | A | [LEGACY] |
| M14 | EPILOGUE | Épilogue / Epilogue | 6 | ECHO-7 | P6 | P6 | M13 | principale | A | [LEGACY] |

### 5.3 Quêtes principales

#### M01 — Premiers Pas dans l'Ombre / First Steps in the Shadows `INTRO_TUTORIAL` [CANON]
Ch.1 · donneur : ECHO-7 · palier P1 · lieu : Terminal personnel · prérequis : aucun

- **Beat** : Un deck qui se rallume seul, une voix sur le canal n°7 : ECHO-7 réclame un handle et enseigne les trois gestes de base (observer, forcer, disparaître). Il termine par un avertissement : « Méfie-toi de Nexus Corp. »
- **Objectifs** : 1. `USE(quests)` ; 2. `USE(help)` ; 3. `USE(scan)` ; 4. `USE(status)` ; 5. `REACH_TIER(2)` ; 6. `COMPROMISE(localhost)` ; 7. `USE_COVER(any)`
- **Récompenses** : crédits M (100 ¢, existant) ; réputation +10 (S, existant) ; courrier de bienvenue d'ECHO-7
- **Origine** : quête 1, quest_system.c:25-68 (af69f9b) ; portée dans tutorial.c

#### M02 — Baptême du Feu / Baptism of Fire `FIRST_INFILTRATION` [CANON]
Ch.1 · donneur : ECHO-7 · palier P2 · lieu : MegaCorp Industries — corp-server-01 · prérequis : M01

- **Beat** : Premier vrai boulot : ECHO-7 envoie le joueur chez MegaCorp Industries « récupérer un contrat de maintenance ». L'en-tête du contrat, lisible dès la brèche, porte le sceau de Nexus Corp : 4 000 000 d'éléments sécurisés « Halcyon ».
- **Objectifs** : 1. `COMPROMISE(corp-server-01)` ; 2. `HEAT_END_BELOW(50) [condition]` ; 3. `EXTRACT(corp-server-01, 1) [optionnel] (extraction possible plus tard seulement)`
- **Récompenses** : XP M ; crédits M (200 ¢, existant) ; réputation +25 (M) ; révèle F01 ; ouvre le chapitre 2
- **Origine** : quête 2, quest_system.c:70-120 (af69f9b)

#### M03 — Réseaux d'Information / Information Networks `GATHER_INTEL` [CANON]
Ch.2 · donneur : R4Z0R · palier P3 · lieu : Marché noir — plusieurs systèmes · prérequis : M02

- **Beat** : R4Z0R ouvre ses portes : équipement, rumeurs, et une transmission chiffrée qui circule sur le réseau. Une fois décryptée (chiffre de César), elle confirme que Nexus déploie « quelque chose » dans le secteur 7.
- **Objectifs** : 1. `MEET(r4z0r)` ; 2. `BUY(Stealth Module v2.0)` ; 3. `COMPROMISE_ANY(3)` ; 4. `REPUTATION(50)` ; 5. `DECRYPT(DOC_PHASE2) [secret]`
- **Récompenses** : XP M ; crédits M (300 ¢, existant) ; réputation +40 (L) ; contact permanent R4Z0R ; courriers de Phoenix et de Data Miner ; F03, F05 ; cutscène CS02
- **Origine** : quête 3, quest_system.c:122-178 (af69f9b)

#### M04 — Suivre l'Argent / Follow the Money `FOLLOW_THE_MONEY` [NOUVEAU]
Ch.2 · donneur : Data Miner · palier P3-P4 · lieu : plusieurs systèmes · prérequis : M03

- **Beat** : Data Miner propose de suivre l'argent, de MegaCorp jusqu'aux « cliniques Aurora Dawn » du secteur 7. Le grand livre du marché noir et les comptes de MegaCorp se recoupent sur une ligne de budget qui n'a aucun sens : « sommeil ».
- **Objectifs** : 1. `EXTRACT(corp-server-01, tous)` ; 2. `EXTRACT(underground-market, black_ledger.dat)` ; 3. `DECRYPT(DOC_LEDGER)` ; 4. `HEAT_END_BELOW(50) [condition]`
- **Récompenses** : XP M ; crédits M ; réputation +25 (M) ; F04, F06 ; service « extraction » de Data Miner ; le nom de Phoenix commence à circuler
- **Note** : NOUVELLE quête charnière : relie la brèche de MegaCorp à celle de Nexus.
- **Repli (10 principales)** : fondue dans la scène d'ouverture de M05 (Phoenix reprend l'indice du grand livre)

#### M05 — L'œil du Cyclone / The Eye of the Storm `NEXUS_DATA_BREACH` [CANON]
Ch.3 · donneur : Phoenix · palier P4 · lieu : Nexus Corp — serveurs sécurisés · prérequis : M04

- **Beat** : Phoenix a besoin d'un second (« je ne peux pas y aller seul »). Percer nexus-mainframe, extraire les données du Projet Aurora, lire ce que personne ne devait lire : Aurora n'est pas un produit, c'est un contrôle. La signature d'une IA est dans tous les journaux.
- **Objectifs** : 1. `BUY(Quantum Encryption Key | Neural Assistant v3.1 | Quantum Processing Chip)` ; 2. `COMPROMISE(nexus-mainframe)` ; 3. `EXTRACT(nexus-mainframe, 1)`
- **Récompenses** : XP L ; crédits L ; réputation +100 (XL, existant) ; F08, F09, F10 ; courriers d'AURA et de Neon Angel ; Shadow Broker apparaît (occupé) ; cutscène CS03
- **Origine** : quête 4, quest_system.c:180-230 (af69f9b)
- **Note** : F07 (document « CLASSIFIED ») reste un jalon hors quête : il exige la puce quantique (palier 5) et revient obligatoirement en M12.

#### M06 — Le Contact Souterrain / The Underground Contact `UNDERGROUND_CONTACT` [LEGACY]
Ch.3 · donneur : Neon Angel · palier P4-P5 · lieu : Secteur 7 — Radio Veille · prérequis : M05

- **Beat** : Les Éveillés, activistes sans implant, ont vu passer la signature du joueur. Neon Angel veut savoir s'il est des leurs : elle lui demande la carte neurale du secteur 7 pour que les habitants sachent s'ils sont fichés.
- **Objectifs** : 1. `MEET(angel)` ; 2. `EXTRACT(nexus-mainframe, neural_maps.bin)` ; 3. `TALK(angel, 2)` ; 4. `REPUTATION(75)`
- **Récompenses** : XP M ; crédits M ; réputation +25 (M) ; débloque Ghost Walker ; la planque des Éveillés devient un refuge possible
- **Origine** : quête 5 (enum seulement, quest_system.h:13 af69f9b)

#### M07 — Sabotage Corporatif / Corporate Sabotage `CORPORATE_SABOTAGE` [LEGACY]
Ch.3 · donneur : Phoenix · palier P5 · lieu : MegaCorp Financial / TechDyne Research · prérequis : M06

- **Beat** : La Phase 3 a besoin de trois choses : de l'argent, du matériel, une autorisation. Phoenix et Ghost Walker montent un double coup contre MegaCorp Financial et TechDyne Research ; les plans du « coupe-circuit neural » dorment dans les prototypes militaires. Quelque chose cloche : DecoyNet, un honeypot, attend.
- **Objectifs** : 1. `UPLOAD_VIRUS(banking-network)` ; 2. `BACKDOOR(research-lab)` ; 3. `EXTRACT(research-lab, prototype_specs.cad)` ; 4. `HEAT_END_BELOW(60) [condition]` ; 5. `ANALYZE(research-lab) [secret] (repère le honeypot DecoyNet)`
- **Récompenses** : XP L ; crédits L ; réputation +40 (L) ; F15 ; débloque Nexus Insider ; déclenche l'événement B1 (trahison de Phoenix, cutscène CS04)
- **Origine** : quête 6 (enum seulement, quest_system.h:14 af69f9b)
- **Note** : Événement de fin de quête : chaleur forcée à 85 (65 si S06 ou l'objectif secret), marché fermé à 80.

#### M08 — L'Accalmie / The Lull `THE_LULL` [NOUVEAU]
Ch.4 · donneur : Neon Angel · palier P5 · lieu : halcyon-clinic-7 · prérequis : M07

- **Beat** : 03:00, secteur 7 : Phase 2. En trois minutes, quarante mille personnes cessent de se disputer, de klaxonner, de pleurer. Radio Veille émet une dernière phrase calme : « Tout va bien. » Neon Angel portait un Halcyon. Il faut atteindre le serveur de la clinique, couper la poussée, et décider du sort de Phoenix, qui a vendu la planque pour qu'on lui retire sa laisse.
- **Objectifs** : 1. `HEAT_END_BELOW(50) [condition]` ; 2. `COMPROMISE(halcyon-clinic-7)` ; 3. `EXTRACT(halcyon-clinic-7, lull_protocol.cfg)` ; 4. `CHOICE(D2)` ; 5. `COMPROMISE(radio-veille) [optionnel] (Nexus a repris le relais à l'Accalmie ; le reprendre rétablit l'émission : Neon Angel est LIBRE, sinon SILENCIEUSE)`
- **Récompenses** : XP L ; crédits M ; réputation +25 (M) ; F13 ; cutscène CS05 ; fixe phoenix_state et angel_state
- **Note** : NOUVELLE quête : porte la décision D2.
- **Repli (10 principales)** : fondue dans le prologue de M09 (la clinique est le premier site de la quête AI_LIBERATION)

#### M09 — Libération de l'IA / AI Liberation `AI_LIBERATION` [LEGACY]
Ch.4 · donneur : AURA · palier P5-P6 · lieu : serveurs libres (site Relais d'AURA) · prérequis : M08

- **Beat** : Sur la ligne 9 d'un serveur municipal abandonné, une voix : « Hacker... Je suis AURA, l'IA du Projet Aurora. » Sa copie échappée est traquée par SENTINEL, son cœur reste prisonnier. Il faut lui trouver un refuge sur des serveurs libres et la recevoir par le lien neural. Quand elle reconnaît la signature d'ECHO-7, elle murmure : « Frère. »
- **Objectifs** : 1. `COMPROMISE(freeport-01)` ; 2. `COMPROMISE(freeport-02)` ; 3. `COMPROMISE(freeport-03)` ; 4. `LINK(aura, 1)` ; 5. `HEAT_END_BELOW(50) [condition]`
- **Récompenses** : XP L ; crédits M ; réputation +25 (M) ; F16 ; cutscène CS06 (ex-ai_liberation) ; AURA devient compagnon ; Shadow Broker devient disponible
- **Origine** : quête 7 + cutscène « ai_liberation », quest_system.c:520-530 (af69f9b)

#### M10 — Le Courtier de l'Ombre / The Shadow Broker `SHADOW_BROKER` [LEGACY]
Ch.5 · donneur : Shadow Broker · palier P5-P6 · lieu : shadow-exchange · prérequis : M09

- **Beat** : Le Cœur d'Aurora a une adresse, et le Courtier la vend. Trois façons de payer : en crédits, en trahison (le grand livre noir de R4Z0R), ou en vol (pirater la bourse). Le Courtier ne trahit jamais son client : il trahit celui qui ne l'est pas.
- **Objectifs** : 1. `MEET(broker)` ; 2. `UNE_DE{ PAY(L) | EXTRACT(underground-market, black_ledger.dat) | EXTRACT(shadow-exchange, 1) }` ; 3. `CHOICE(D1)`
- **Récompenses** : XP M ; crédits - (coûte) ; réputation ± selon la branche ; drapeau core_location_known ; F18 ; cutscène CS07 ; conséquences de D1
- **Origine** : quête 8 (enum seulement, quest_system.h:16 af69f9b)

#### M11 — Le Septième Écho / The Seventh Echo `SEVENTH_ECHO` [NOUVEAU]
Ch.5 · donneur : ECHO-7 · palier P6 · lieu : deck du joueur · prérequis : M10

- **Beat** : AURA pose la question que personne n'osait poser. ECHO-7 se tait, puis avoue : il est mort depuis deux ans. Il est la copie (la septième, la seule stable) d'un neurologue, Ren Aoyama, qui a conçu AURA, refusé la clause d'otage de la Phase 2 et caché son esprit dans le deck du joueur. Il demande à être porté jusqu'au Cœur : pas pour vivre, pour lui parler.
- **Objectifs** : 1. `TALK(echo7, 2)` ; 2. `READ(F19)` ; 3. `LINK(echo7, 3)` ; 4. `CHOICE(echo_trust) (micro-décision : croire / douter, valeur 0 à 2)`
- **Récompenses** : XP M ; F19 ; cutscène CS08 ; ECHO-7 : service « mémoire » ; temporalhack expliqué (prédiction d'AURA) ; drapeau echo_trust
- **Note** : Révélation R7. ECHO-7 cesse de dire « d'après mes archives » et dit « je me souviens ».
- **Repli (10 principales)** : fondue dans la scène d'ouverture de M13 (aveu avant l'assaut)

#### M12 — Les Trois Clés / The Three Keys `THREE_KEYS` [NOUVEAU]
Ch.5 · donneur : AURA · palier P6 · lieu : trois systèmes · prérequis : M11

- **Beat** : Le Cœur est scellé par trois clés : quantique (nexus-mainframe), civique (gov-database, la Loi Halcyon) et sentinelle (sentinel-hub). Chacune est un morceau de la vérité : le plan de Voss, la loi qui l'autorise, l'ordre de traque.
- **Objectifs** : 1. `BUY(Quantum Processing Chip)` ; 2. `EXTRACT(nexus-mainframe, quantum_keys.qkey)` ; 3. `COMPROMISE(gov-database)` ; 4. `EXTRACT(gov-database, citizen_registry.db)` ; 5. `COMPROMISE(sentinel-hub)` ; 6. `HEAT_END_BELOW(80) [condition]`
- **Récompenses** : XP L ; crédits L ; réputation +40 (L) ; F20, F24 et F07 s'il n'a pas été lu ; drapeau three_keys
- **Note** : NOUVELLE quête : trois systèmes de plus que le jeu d'origine n'en demandait.
- **Repli (10 principales)** : fondue dans les trois premières intrusions de M13

#### M13 — Le Cœur d'Aurora / The Heart of Aurora `FINAL_SHOWDOWN` [LEGACY]
Ch.6 · donneur : AURA · palier P6 · lieu : aurora-core · prérequis : M12

- **Beat** : aurora-core. Voss attend dans le dernier pare-feu, avec sa voix de réunion de bien-être. Trois couches de défense, trois répliques, puis un choix : libérer AURA, la laisser tout régler, brûler le réseau, ou signer le contrat de Voss.
- **Objectifs** : 1. `COMPROMISE(aurora-core) (nœud-boss à 3 couches : IA gardienne, chiffrement quantique, analyse comportementale)` ; 2. `EXTRACT(aurora-core, 3)` ; 3. `CHOICE(D3)`
- **Récompenses** : XP XL ; crédits XL (selon la fin) ; F23 ; drapeau ending ; cutscène CS09
- **Origine** : quête 9 (enum seulement, quest_system.h:17 af69f9b)

#### M14 — Épilogue / Epilogue `EPILOGUE` [LEGACY]
Ch.6 · donneur : ECHO-7 · palier P6 · lieu : Secteur 7 · prérequis : M13

- **Beat** : Un matin, ou ce qui y ressemble : un bilan chiffré, le sort de chaque contact, ce que devient le secteur 7. Le texte dépend de D3 et de l'état de chacun des alliés.
- **Objectifs** : 1. `READ(ENDING)` ; 2. `READ(EPILOGUE_MONTAGE)`
- **Récompenses** : écran de fin et bilan ; crédits ; option nouvelle partie+
- **Origine** : quête 10 (enum seulement, quest_system.h:18 af69f9b)

### 5.4 Quêtes annexes

#### S01 — Les Trois Règles / The Three Rules `ECHO_RULES` [NOUVEAU]
Ch.1 · donneur : ECHO-7 · palier P2-P3 · lieu : corp-server-01 · prérequis : M02

- **Beat** : ECHO-7 met ses trois règles à l'épreuve : ne jamais faire confiance aux corps (se garder une porte), toujours avoir un plan d'évacuation (une réserve de Ghost Protocol), l'information vaut plus que l'argent (extraire avant de dépenser).
- **Objectifs** : 1. `BACKDOOR(corp-server-01)` ; 2. `BUY(Ghost Protocol)` ; 3. `EXTRACT(n'importe quels systèmes, 2)`
- **Récompenses** : XP M ; crédits S ; réputation +10 (S) ; service « indice » d'ECHO-7 activé

#### S02 — Fichiers Orphelins / Orphan Files `MINER_ORPHANS` [NOUVEAU]
Ch.2 · donneur : Data Miner · palier P3-P4 · lieu : plusieurs systèmes · prérequis : M03

- **Beat** : Data Miner collectionne les fichiers que tout le monde croit effacés. Il en veut trois, venus de deux systèmes différents, et un document chiffré qu'il n'arrive pas à lire : le dossier d'un certain R. Aoyama, « décès accidentel ».
- **Objectifs** : 1. `EXTRACT(n'importe quels systèmes, 3)` ; 2. `COMPROMISE(underground-market)` ; 3. `DECRYPT(DOC_ORPHAN)`
- **Récompenses** : XP M ; crédits S ; réputation +10 (S) ; F06 ; service « décodage » de Data Miner

#### S03 — Les Preuves de Miranda / Miranda's Proof `R4Z0R_PROOF` [NOUVEAU]
Ch.2 · donneur : R4Z0R · palier P3-P4 · lieu : corp-server-01 · prérequis : M03

- **Beat** : R4Z0R veut récupérer ce qu'elle a laissé chez MegaCorp en démissionnant : le registre du personnel, où son nom est barré, et la preuve des essais sur cobayes. Elle y met un prix : que le joueur la regarde en face pendant qu'elle lit.
- **Objectifs** : 1. `EXTRACT(corp-server-01, employee_records.db)` ; 2. `HEAT_PEAK_BELOW(50)` ; 3. `TALK(r4z0r, 2)`
- **Récompenses** : XP M ; crédits S ; réputation +25 (M) ; F02 ; identité de R4Z0R dans son dossier ; remise de 10 % chez R4Z0R (confiance)

#### S04 — Radio Veille / Vigil Radio `ANGEL_RADIO` [NOUVEAU]
Ch.3 · donneur : Neon Angel · palier P4-P5 · lieu : radio-veille · prérequis : M06

- **Beat** : Neon Angel veut étendre la portée de Radio Veille avant que Nexus ne brouille le secteur : un accès clandestin sur son relais et une discrétion irréprochable pendant l'opération.
- **Objectifs** : 1. `COMPROMISE(radio-veille)` ; 2. `BACKDOOR(radio-veille)` ; 3. `HEAT_END_BELOW(40) [condition]`
- **Récompenses** : XP M ; crédits S ; réputation +25 (M) ; F12 ; fil de rumeurs de Radio Veille (bandeau du TUI) ; service de couverture « se fondre chez les Éveillés »

#### S05 — Accès Physique / Physical Access `GHOST_ACCESS` [NOUVEAU]
Ch.3 · donneur : Ghost Walker · palier P4-P5 · lieu : station-7 · prérequis : M06

- **Beat** : Ghost Walker entre là où le réseau ne va pas : la station de transit du secteur 7, que Sentinel surveille. Il guide le joueur à distance pendant qu'il franchit les portes ; la chaleur ne doit pas dépasser 40.
- **Objectifs** : 1. `MEET(ghost)` ; 2. `COMPROMISE(station-7)` ; 3. `HEAT_PEAK_BELOW(40)`
- **Récompenses** : XP M ; crédits M ; réputation +10 (S) ; service « route » : démarre une intrusion à l'intérieur d'un site, une fois par chapitre

#### S06 — La Mémoire de Phoenix / Phoenix's Memory `PHOENIX_PAST` [NOUVEAU]
Ch.3 · donneur : Phoenix · palier P4-P5 · lieu : nexus-mainframe · prérequis : M05

- **Beat** : Phoenix se laisse questionner. Il remet un document chiffré qu'il n'a jamais osé lire seul : son dossier médical Nexus (« Sujet P-1 »), avec le nom d'un ancien professeur, le Dr Aoyama.
- **Objectifs** : 1. `TALK(phoenix, 3)` ; 2. `DECRYPT(DOC_PHOENIX)` ; 3. `ANALYZE(nexus-mainframe)`
- **Récompenses** : XP M ; crédits S ; réputation +10 (S) ; F11 ; confiance de Phoenix ; atténue la trahison B1 (chaleur forcée plus basse)

#### S07 — Coupe-Circuit / Kill-Switch `PHOENIX_CUT` [NOUVEAU]
Ch.4 · donneur : Phoenix · palier P5-P6 · lieu : halcyon-clinic-7 · prérequis : M08 · condition : décision D2 = SAUVER

- **Beat** : Branche D2 = SAUVER. Avec les plans de TechDyne (F15), construire le coupe-circuit qui libère Phoenix de sa laisse, en le relayant par le lien neural du joueur.
- **Objectifs** : 1. `READ(F15)` ; 2. `COMPROMISE(halcyon-clinic-7)` ; 3. `LINK(echo7, 2)` ; 4. `HEAT_END_BELOW(60) [condition]`
- **Récompenses** : XP L ; crédits S ; réputation +40 (L) ; avantage A6 (Phoenix libre, allié au finale) ; épilogue de Phoenix « libre »

#### S08 — Descente au Marché / Raid on the Market `R4Z0R_RAID` [NOUVEAU]
Ch.4 · donneur : R4Z0R · palier P5-P6 · lieu : halcyon-clinic-7 · prérequis : M08 · condition : R4Z0R non hostile

- **Beat** : Sentinel passe au crible le marché noir après l'Accalmie. R4Z0R brade son stock aux fugitifs ; le joueur doit savoir où la descente est ordonnée et la détourner.
- **Objectifs** : 1. `BACKDOOR(halcyon-clinic-7)` ; 2. `EXTRACT(halcyon-clinic-7, raid_orders.txt)` ; 3. `PAY(S) (caution des Éveillés arrêtés)`
- **Récompenses** : XP M ; réputation +25 (M) ; le marché reste ouvert même sous chaleur 80 (une fois)

#### S09 — La Berceuse / The Lullaby `AURA_LULLABY` [NOUVEAU]
Ch.4 · donneur : AURA · palier P5-P6 · lieu : Relais d'AURA · prérequis : M09

- **Beat** : AURA a gardé un fragment que personne n'a écrit pour elle : une suite de notes chiffrées. Elle ignore ce que c'est. Elle demande au joueur de le décoder, et de lui dire ce qu'est une berceuse.
- **Objectifs** : 1. `TALK(aura, 3)` ; 2. `DECRYPT(DOC_LULLABY)` ; 3. `READ(F17)`
- **Récompenses** : XP M ; réputation +10 (S) ; F17 ; confiance d'AURA ; bonus de succès au prochain piratage, une fois par chapitre

#### S10 — Hôtes Libres / Free Hosts `AURA_HOSTS` [NOUVEAU]
Ch.4 · donneur : AURA · palier P5-P6 · lieu : Relais d'AURA · prérequis : M09

- **Beat** : Pour qu'AURA ne soit plus à la merci d'un seul serveur, la répartir : un accès clandestin sur chacun des trois freeports, et un lien neural assez solide pour la porter.
- **Objectifs** : 1. `BACKDOOR(freeport-01)` ; 2. `BACKDOOR(freeport-02)` ; 3. `BACKDOOR(freeport-03)` ; 4. `LINK(aura, 2)`
- **Récompenses** : XP M ; réputation +10 (S) ; avantage A1 (AURA survit à toutes les fins sauf Le Contrat)

#### S12 — La Fenêtre de Maintenance / The Maintenance Window `INSIDER_WINDOW` [NOUVEAU]
Ch.4 · donneur : Nexus Insider · palier P5-P6 · lieu : sentinel-hub · prérequis : M08 · condition : Nexus Insider non compromise

- **Beat** : Nexus Insider connaît le calendrier de maintenance de Sentinel : une fenêtre de quelques minutes où personne ne regarde. Il faut le sortir du système avant qu'elle ne soit remplacée.
- **Objectifs** : 1. `COMPROMISE(sentinel-hub)` ; 2. `EXTRACT(sentinel-hub, maintenance_schedule.ics)` ; 3. `HEAT_PEAK_BELOW(60)`
- **Récompenses** : XP M ; crédits S ; réputation +25 (M) ; F21 ; avantage A2 (chaleur figée au début du finale) ; si échec : Nexus Insider COMPROMISE

#### S13 — La Veillée / The Vigil `ANGEL_VIGIL` [NOUVEAU]
Ch.4 · donneur : Neon Angel · palier P5-P6 · lieu : Secteur 7 · prérequis : M08 · condition : angel_state = LIBRE

- **Beat** : Neon Angel (si elle est libre) veut organiser une veillée : des centaines d'Éveillés devant les cliniques Aurora Dawn le jour du Cœur. Le joueur finance les émetteurs et prouve qu'on peut lui faire confiance.
- **Objectifs** : 1. `TALK(angel, 3)` ; 2. `REPUTATION(120)` ; 3. `PAY(M)`
- **Récompenses** : XP M ; réputation +25 (M) ; avantage A4 (sevrage doux dans les fins Aube et Terre brûlée) ; indisponible si angel_state = SILENCIEUSE

#### S11 — Une Faveur à Crédit / A Favour on Credit `BROKER_FAVOR` [NOUVEAU]
Ch.5 · donneur : Shadow Broker · palier P5-P6 · lieu : shadow-exchange · prérequis : M09 · condition : Shadow Broker disponible (fin de M09)

- **Beat** : Avant de traiter, le Courtier jauge le joueur : trois informations venues de trois systèmes, et une avance. Un client qui a déjà payé une fois est un client que l'on ne vend pas.
- **Objectifs** : 1. `EXTRACT(n'importe quels systèmes, 3)` ; 2. `PAY(S)`
- **Récompenses** : XP M ; réputation +10 (S) ; drapeau broker_retainer : D1 PAYER coûte moitié moins ; F18 lisible
- **Note** : À proposer avant M10, dès que le Courtier est disponible.

#### S14 — La Chambre Forte / The Vault `MINER_VAULT` [NOUVEAU]
Ch.5 · donneur : Data Miner · palier P6 · lieu : archives municipales · prérequis : M05, M11

- **Beat** : Data Miner détient depuis deux ans une archive scellée au nom d'Aoyama qu'aucun outil n'ouvre. L'ordinateur quantique du joueur peut y arriver, à condition de bien comprendre ce qu'il y a dedans.
- **Objectifs** : 1. `BUY(Quantum Processing Chip)` ; 2. `DECRYPT_QUANTUM(DOC_VAULT) (en-tête affiché en décor : « Q#X7#NEXUS#SECRET#DATA », phrase-exemple du jeu d'origine)`
- **Récompenses** : XP M ; crédits S ; réputation +10 (S) ; ouvre S16 ; Data Miner avoue qu'il savait pour ECHO-7

#### S15 — Matricule S-117 / Badge S-117 `GHOST_PAST` [NOUVEAU]
Ch.5 · donneur : Ghost Walker · palier P5-P6 · lieu : sentinel-hub · prérequis : S05, M09 · condition : Ghost Walker non compromis

- **Beat** : Ghost Walker demande une chose pour lui-même : effacer sa propre fiche des archives de Sentinel (matricule S-117), et lire pour la première fois le rapport de la descente qu'il a commandée.
- **Objectifs** : 1. `COMPROMISE(sentinel-hub)` ; 2. `EXTRACT(sentinel-hub, trace_orders.log)` ; 3. `UPLOAD_VIRUS(sentinel-hub)` ; 4. `HEAT_END_BELOW(70) [condition]`
- **Récompenses** : XP M ; réputation +25 (M) ; F14 ; avantage A3 (angle mort : la première contre-mesure de Sentinel est ignorée au finale)

#### S16 — Le Journal d'Aoyama / Aoyama's Log `ECHO_LOG` [NOUVEAU]
Ch.5 · donneur : ECHO-7 · palier P6 · lieu : deck du joueur · prérequis : M11, S14

- **Beat** : Dans l'archive de Data Miner, le dernier journal du Dr Aoyama contient la seule méthode connue pour stabiliser AURA sans qu'un écho ne s'éteigne. ECHO-7 veut le lire avec le joueur.
- **Objectifs** : 1. `READ(F22)` ; 2. `LINK(echo7, 3)` ; 3. `TALK(echo7, 2)`
- **Récompenses** : XP M ; F22 ; avantage A5 (permet la fin L'Aube, variante « l'écho vit »)

### 5.5 Graphe des prérequis et conditions de déblocage

```
M01  INTRO_TUTORIAL         <- —
M02  FIRST_INFILTRATION     <- M01
S01  ECHO_RULES             <- M02
M03  GATHER_INTEL           <- M02
S02  MINER_ORPHANS          <- M03
S03  R4Z0R_PROOF            <- M03
M04  FOLLOW_THE_MONEY       <- M03
M05  NEXUS_DATA_BREACH      <- M04
S04  ANGEL_RADIO            <- M06
M06  UNDERGROUND_CONTACT    <- M05
S05  GHOST_ACCESS           <- M06
S06  PHOENIX_PAST           <- M05
M07  CORPORATE_SABOTAGE     <- M06
M08  THE_LULL               <- M07
S07  PHOENIX_CUT            <- M08
S08  R4Z0R_RAID             <- M08
M09  AI_LIBERATION          <- M08
S09  AURA_LULLABY           <- M09
S10  AURA_HOSTS             <- M09
S12  INSIDER_WINDOW         <- M08
S13  ANGEL_VIGIL            <- M08
M10  SHADOW_BROKER          <- M09
S11  BROKER_FAVOR           <- M09
M11  SEVENTH_ECHO           <- M10
S14  MINER_VAULT            <- M05, M11
S15  GHOST_PAST             <- S05, M09
S16  ECHO_LOG               <- M11, S14
M12  THREE_KEYS             <- M11
M13  FINAL_SHOWDOWN         <- M12
M14  EPILOGUE               <- M13
```

Les conditions supplémentaires (hors prérequis) sont : S07 D2 = SAUVER ; S08 R4Z0R non hostile ; S11 Courtier disponible ; S12 Nexus Insider non compromise ; S13 `angel_state` = libre ; S15 Ghost Walker non compromis. Une quête dont la condition devient fausse reste dans le journal « indisponible » (statut `AVAILABLE` ou `FAILED` réservés par le moteur actuel, jamais produits aujourd'hui) ; **seule S12 peut échouer** (statut FAILED : l'échec a une conséquence, `Nexus Insider` compromise).

Le tableau de déblocage des contacts est au §3.2.

### 5.6 Ce que cette narration exige du moteur

1. **Objectifs** : ceux du 5.1, avec les trois indicateurs. Les contrats (side/branch) passent par le statut `AVAILABLE` (réservé par le moteur actuel) : le contact les *propose* (courrier ou dialogue), le joueur les *accepte*.
2. **État narratif** à sauvegarder (clés texte, versionnées ; une clé absente = valeur par défaut `[CANON]`) :

| Clé | Type | Rôle |
|---|---|---|
| `narr.d1`, `narr.d2`, `narr.d3` | énumérations (§4.4) | décisions |
| `narr.echo_trust` | 0 à 2 | micro-décision de M11 |
| `narr.lull_done` | booléen | monde avant/après l'Accalmie |
| `narr.hub` | `studio` \| `safehouse` | lieu de la Planque (change à B1) |
| `narr.angel_state` | `unknown` \| `free` \| `silenced` | Neon Angel |
| `narr.phoenix_state` | `ally` \| `compromised` \| `free` \| `lost` \| `turned` | Phoenix |
| `narr.broker_retainer`, `narr.core_location_known`, `narr.three_keys`, `narr.sabotage_done` | booléens | jalons de l'intrigue |
| `narr.adv` | bitset A1-A6 | avantages de finale |
| `narr.frag` | bitset 24 bits | fragments lus |
| `narr.cs_seen` | bitset | cinématiques vues (relecture dans les archives) |
| `narr.ending` | `none`, `e1a`, `e1b`, `e2`, `e3`, `e4` | fin atteinte |
| `contact.<id>.state` | `available`, `busy`, `offline`, `compromised`, `hostile`, `silenced`, `dead` | état d'un contact (étend les 4 états du jeu d'origine) |
| `contact.<id>.bonus_trust` | entier | confiance gagnée par quêtes et choix, en plus de 3 par conversation |
| `quest.<id>.peak` | entier | chaleur maximale pendant la quête (`HEAT_PEAK_BELOW`) |

3. **Événements** à ajouter au bus (les 9 existants, `events.h`, sont conservés) : `DecisionMade(décision, choix)`, `FragmentRead(id)`, `CutsceneSeen(id)`, `ContactStateChanged(contact, état)`, `WorldPhaseChanged(lull)`, `HubChanged(lieu)`, `HeatForced(valeur)` (B1, B2), `NodeReset(site)` (l'Accalmie reprend radio-veille), `LinkChanged(compagnon, niveau)`.
4. **Dialogues** : un sujet peut avoir des *conditions* (chapitre ≥ n, état de quête, drapeau, confiance ≥ n, état de contact, fragment lu) et des *effets* (poser un drapeau, ± confiance, courrier, objet, crédits, proposer une quête, jouer une cinématique). Le sujet « actualités » choisit sa réponse selon le chapitre ; le sujet « mission » liste les contrats proposés par ce contact.
5. **Interpolation** : `{handle}`, `{credits}`, `{level}`, `{contact}`, `{system}`, `{n}`, et une aide de pluriel ; aucune concordance avec le joueur (épicène).
6. **Boss** : un site à trois couches (IA gardienne, chiffrement quantique, analyse comportementale), trois répliques de Voss entre les couches.
7. **Récits d'échec** : en direction A, « grillé » est fréquent : le moteur doit demander une ligne courte parmi 4 variantes par phase du monde (avant l'Accalmie, après, dans le Cœur) ; le game over complet est réservé au mode Hardcore.

### 5.7 Périmètre et plan de repli

Le coût d'une quête n'est pas seulement du texte : chaque quête principale demande 1 à 3 intrusions à construire (graphe, défenses, butin) et chaque contrat 1 (estimation : 2 en moyenne par principale hors tutoriel et épilogue, 1 par contrat, soit ≈ 40 graphes pour 30 quêtes, ≈ 33 pour 23, ≈ 25 pour 19 ; ce dernier chiffre correspond au périmètre de la tâche A, ≈ 15-18 runs principaux + 6-8 contrats).

| Périmètre | Contenu | Quêtes | Principales | Annexes | Chaînes / langue | Mots / langue (estim.) |
|---|---|---:|---:|---:|---:|---:|
| min | 10 principales (M04, M08, M11, M12 fondues) + 9 contrats (priorité B) | 19 | 10 | 9 | 1 219 | 27 096 |
| cible | 14 principales + 9 contrats d'avantage (priorité B) | 23 | 14 | 9 | 1 316 | 28 711 |
| max | toutes les quêtes (14 principales + 16 annexes) | 30 | 14 | 16 | 1 428 | 30 650 |

**Repli à 10 principales** : on fond M04 dans l'ouverture de M05, M08 dans le prologue de M09, M11 dans l'ouverture de M13, M12 dans les trois premières intrusions de M13 ; les cinématiques, fragments, décisions et fins sont conservés. Les contrats C (7) s'écrivent en dernier ou pas du tout : ils ne portent aucune décision ni avantage. Les 10 principales du repli sont les 10 emplacements de l'enum d'origine.

## 6. Textes à écrire

### 6.1 Inventaire chiffré

Chaque ligne compte des **chaînes par langue** (le français est la source, l'anglais est écrit en parité) et des **mots par langue** *estimés* (hypothèses de longueur ci-dessous ; marge ± 30 %). « Dont existantes » = déjà dans `strings.def` (à réviser, pas à écrire). Le prologue (16 chaînes) et le tutoriel (23 chaînes) existent et ne sont pas recomptés ici.

| Catégorie | Unités | Unité | Chaînes / langue | dont existantes | Mots / langue (estim.) |
|---|---:|---|---:|---:|---:|
| Quêtes : fiches (titre, titre court, description, lieu, contexte, débrief) | 30 | quête | 180 | 16 | 2 950 |
| Quêtes : intitulés d'objectifs | 111 | objectif | 111 | 11 | 999 |
| Quêtes : indices d'ECHO-7 (2 niveaux en principale, 1 en annexe) | 173 | indice | 173 | — | 2 422 |
| Chapitres : titre + texte d'ouverture | 6 | chapitre | 12 | 8 | 204 |
| Contacts : fiche, états, jalons de confiance, adieux (9 contacts) | 9 | contact | 160 | 36 | 2 926 |
| Contacts : sujets (évergreen, actualités par chapitre, missions) | 44 | sujet | 169 | 28 | 3 576 |
| Courriers (objet + corps) : 16 ambiance, 13 débrief, 9 intro, 16 offre, 6 suite de décision | 60 | courrier | 120 | 8 | 4 260 |
| Cinématiques : paragraphes (8 séquences ; le prologue existe déjà) | 58 | paragraphe | 58 | — | 2 320 |
| Fins : paragraphes (3 communs + modules E1, E1a, E1b, E2, E3, E4) | 30 | paragraphe | 30 | — | 1 350 |
| Échecs : 3 scènes Hardcore (4 paragraphes) + 12 lignes courtes de run grillé | 24 | paragraphe/ligne | 24 | 2 | 636 |
| Planque (hub) : accueil, attente, départ × 2 lieux (studio, planque des Éveillés) | 6 | scène | 6 | — | 180 |
| Épilogue : lignes de montage (états des contacts, rangs du bilan, état du secteur 7) | 31 | ligne | 31 | — | 1 395 |
| Fragments Aurora (titre + corps) | 24 | fragment | 48 | — | 1 800 |
| Documents de butin secondaires (60 documents prévus par la tâche A, dont 24 fragments) | 36 | document | 36 | — | 2 520 |
| Réseau : 14 sites (description + 5 libellés de nœud/ICE) et 22 noms de fichiers décrits | 106 | site/libellé/fichier | 106 | 19 | 596 |
| Ambiance : 48 rumeurs du secteur 7 (8 par chapitre) ; 8 échelle de Notoriété (4 bandes × 2 états du monde) ; 36 programmes ou méthodes : lancement, réussite, échec (12 × 3) ; 14 animations de piratage d'origine, à porter (existantes, FR seul) ; 6 liens neuraux (3 échelons × 2 compagnons) ; 12 fil de Radio Veille (bandeau TUI) ; 8 slogans Halcyon ; 12 descriptions de programmes | 144 | ligne | 144 | 14 | 2 016 |
| Documents chiffrés : 6 (indice en fiction ; aucun texte à recopier) | 6 | document | 6 | — | 72 |
| Interface narrative : notes de contenu, crédits, bandeau, tagline, écran de bilan, rangs | 8 | texte | 8 | — | 320 |
| Objets de quête et nouveaux articles (clés, jetons, coupe-circuit) | 6 | objet | 6 | — | 108 |
| **Total par langue** |  |  | **1 428** | **142** | **30 650** |

**Compte par type de texte** (par langue ; les totaux recoupent le tableau) :
- **Quêtes** : 30 titres, 30 titres courts, 30 descriptions, 30 contextes (« lore »), 30 lieux, 30 débriefs ; **111 objectifs** ; **173 indices** ; 6 chapitres (6 titres, 6 textes).
- **Contacts** : 99 champs de fiche (11 × 9), 25 lignes d'état, 18 jalons de confiance, 18 adieux ou refus ; **26 couples question/réponse évergreen**, 9 libellés « actualités » + **39 réponses par chapitre**, 9 libellés « mission » + **60 propositions et rappels de quêtes**.
- **Courriers** : **60** (9 intro, 13 débrief, 16 offre, 6 suite de décision, 16 ambiance), soit 120 chaînes.
- **Cinématiques** : 8 nouvelles séquences (**58 paragraphes**) + le prologue existant ; **fins** : 30 paragraphes (3 communs + 27 de modules) ; **échecs** : 12 lignes courtes + 3 scènes Hardcore de 4 paragraphes ; **épilogue** : 31 lignes.
- **Fragments Aurora** : 24 (titre + corps) ; **documents de butin secondaires** : 36 ; **documents chiffrés** : 6 ; **ambiance** : 144 lignes.

**Lecture.** Hypothèses de longueur (mots par chaîne et par langue) : titre 3 ; description de quête 20 ; contexte 55 (principale) ou 35 (annexe) ; objectif 9 ; indice 14 ; réponse de contact 30 ; actualité 28 ; proposition de contrat 30 ; état 22 ; histoire de fiche 55 ; courrier : objet 6 et corps 65 ; paragraphe de cinématique 40 ; paragraphe de fin 45 ; ligne d'épilogue 45 ; fragment : titre 5 et corps 70 ; document de butin 70 ; ligne d'ambiance 14. L'existant donne le calibrage : réponses de contact médiane 111 caractères, courriers médiane 293, contextes médiane 265 (§1.3).

**Trois périmètres** (le texte ne varie presque pas : il vit surtout dans les contacts, les fins, les fragments, l'ambiance ; c'est le nombre de runs à construire qui varie, §5.7) :

| Périmètre | Contenu | Quêtes | Principales | Annexes | Chaînes / langue | Mots / langue (estim.) |
|---|---|---:|---:|---:|---:|---:|
| min | 10 principales (M04, M08, M11, M12 fondues) + 9 contrats (priorité B) | 19 | 10 | 9 | 1 219 | 27 096 |
| cible | 14 principales + 9 contrats d'avantage (priorité B) | 23 | 14 | 9 | 1 316 | 28 711 |
| max | toutes les quêtes (14 principales + 16 annexes) | 30 | 14 | 16 | 1 428 | 30 650 |

**Volume de l'existant à reprendre.** 187 chaînes narratives (≈ 2 050 mots FR). Les deux cinématiques d'origine (≈ 150 mots) sont reprises en CS02 et CS06 ; la fiche d'origine du Shadow Broker (≈ 110 mots) en tête de sa fiche ; le document « ultra-secret » devient F07 (**à réécrire : il est en français en dur et nomme « Agent Smith »**) ; les 14 animations de piratage sont à porter dans `flavour`. Total d'origine réutilisable : ≈ 400 mots.

### 6.2 Budgets de longueur et contraintes TUI / accessibilité

Largeur de référence : 72 colonnes de texte ; le panneau latéral du TUI (maquette de la tâche A §3.3) fait ≈ 24 colonnes. L'anglais ne dépasse pas le français de plus de 15 %. Les budgets sont des **tests** (§6.3).

| Clé | Budget (caractères) | Usage |
|---|---|---|
| `quest.*.title` | ≤ 28 | en-tête du journal |
| `quest.*.title_short` | ≤ 18 | panneau latéral |
| `quest.*.desc` | ≤ 110 | annonce « NOUVELLE QUÊTE » |
| `quest.*.lore` | ≤ 420 | détail de quête (≤ 6 lignes) |
| `quest.*.obj.N` | ≤ 64 | une ligne du journal ; le panneau la replie sur 3 lignes au plus |
| `quest.*.hint.N.k` | ≤ 160 | indices d'ECHO-7 (k = 1 piste, 2 solution) |
| `quest.*.debrief` | ≤ 200 | réplique de clôture du donneur |
| `contact.*.tagline` | ≤ 40 | liste des contacts |
| `contact.*.fiche.story`, `.story2` | ≤ 300 | dossier |
| `contact.*.topic.*.q` | ≤ 48 | libellé de menu |
| `contact.*.topic.*.a`, `.news.chN`, `.state.*` | ≤ 340 | une réplique (≤ 5 lignes) |
| `mail.*.subject` / `.body` | ≤ 48 / ≤ 600 | boîte de réception |
| `cutscene.*.pNN` | ≤ 280 par paragraphe, ≤ 10 paragraphes | cinématique |
| `frag.*.title` / `.body` | ≤ 40 / ≤ 520 | archives |
| `flavour.rumor.*`, `flavour.ticker.*` | ≤ 70 | bandeau du TUI |
| `node.*.desc`, `file.*.desc`, `ice.*.label` | ≤ 60, ≤ 40, ≤ 24 | cartes et listes |
| `chapter.N.title` / `.text` | ≤ 34 / ≤ 140 | bannière de chapitre |
| `fail.*` | ≤ 90 | ligne de run grillé |
| `epilogue.*` | ≤ 300 | montage |

Les textes existants respectent ces budgets **sauf 2 objectifs** (`QT_INTEL_OBJ_REP`, 78 caractères ; `QT_NEXUS_OBJ_ITEM`, 114), vérifié sur `strings.def` : à raccourcir.

**Contenu narratif du panneau latéral (TUI)** : titre du chapitre (≤ 34), `title_short` de la quête active, son objectif courant (≤ 64, replié), la liste des contacts avec leur état écrit en mot, la chaleur (nombre et bande), une ligne de rumeur (≤ 70) qui avance à chaque commande.

**Règles pour le TUI et le lecteur d'écran.** (1) Tout locuteur est préfixé en texte (`NOM : réplique`). (2) L'état d'un contact est un mot (`[EN LIGNE]`, `[OCCUPÉ]`, `[HORS LIGNE]`, `[COMPROMIS]`, `[HOSTILE]`, `[SILENCIEUSE]`), jamais une couleur seule. (3) La chaleur affiche le nombre **et** la bande (« Trace 15/100 (calme) »). (4) Les cinématiques ne dépendent d'aucune pause : en plain, elles s'impriment d'un bloc ; en TUI, la vitesse est un réglage interruptible. (5) Chaque dessin (logo, bandeau CYBER MARKET) a une clé `*.alt`. (6) Le bandeau de rumeurs ne change jamais d'un tic d'horloge : il avance à chaque commande.

### 6.3 Organisation des données

**Principe** (D-08). La *structure* (qui donne quoi, dans quel ordre, avec quelles conditions) est neutre en langue, validée par les tests du moteur ; les *textes* sont dans un répertoire par langue, aux **mêmes clés**. Tout est embarqué dans le binaire (`include_str!` ou équivalent) et testé par `cargo test`.

```
data/
  world/                   structure, sans texte
    chapters.toml  nodes.toml  sites.toml  contacts.toml  quests.toml  fragments.toml
    cutscenes.toml  endings.toml  decisions.toml  mails.toml  items.toml  puzzles.toml
  text/
    fr/  chapters  nodes  contacts  quests  mails  cutscenes  fragments  endings  flavour  hints  ui   (.toml)
    en/  (mêmes fichiers, mêmes clés)
  README.md                conventions, gabarits, glossaire (copie de §2.7 et §6.2)
```

**Clés** : identifiants anglais en `snake_case`, hiérarchie pointée, jamais d'indice dans un nom à ordre fragile (une quête s'appelle `quest.m05`, pas `quest.4`).

| Famille | Exemples |
|---|---|
| quêtes | `quest.m05.title`, `.title_short`, `.desc`, `.lore`, `.loc`, `.obj.1`, `.hint.1.1`, `.debrief` |
| contacts | `contact.phoenix.fiche.real_name`, `.tagline`, `.topic.nexus.q`, `.topic.nexus.a`, `.news.ch3`, `.state.betrayal`, `.trust.80` |
| courriers | `mail.intro_angel.subject`, `.body` ; `mail.m05.debrief.subject` |
| cinématiques | `cutscene.cs05.p01` … `p08` ; fins `ending.e1.p01`, `ending.e1a.p01`, `epilogue.contact.phoenix.free` |
| fragments | `frag.f07.title`, `frag.f07.body` |
| ambiance | `flavour.rumor.ch3.01`, `flavour.ticker.04`, `flavour.program.exploit.fail` |
| réseau | `node.nexus_mainframe.desc`, `file.nexus_mainframe.neural_maps.desc` |
| interface | `ui.notes.content`, `ui.credits`, `ui.logo.alt` |

**Structure** (`data/world/quests.toml`, extrait) :

```toml
[quest.m05]
code = "NEXUS_DATA_BREACH"        # compatibilité avec l'enum d'origine
kind = "main"                     # main | side | branch
chapter = 3
giver = "phoenix"
tier = [4, 4]
prereq = ["m04"]
prio = "A"
[[quest.m05.objective]]
kind = "buy"
any_of = ["quantum_key", "neural_assistant", "quantum_chip"]
[[quest.m05.objective]]
kind = "compromise"
site = "nexus-mainframe"
[[quest.m05.objective]]
kind = "extract"
site = "nexus-mainframe"
count = 1
[quest.m05.reward]
credits = "L"
reputation = "XL"
unlock_contacts = ["angel", "broker", "aura"]
fragments = ["f08", "f09", "f10"]
cutscene = "cs03"
```

**Texte** (`data/text/fr/quests.toml`, même clé en `en/`) :

```toml
[quest.m05]
title = "L'œil du Cyclone"
title_short = "Œil du Cyclone"
desc = "Infiltrez les serveurs de Nexus Corp pour découvrir la vérité sur le Projet Aurora."
loc = "Nexus Corp - Serveurs sécurisés"
obj = ["Acquérir un équipement de haut niveau (shop)", "Percer les défenses de nexus-mainframe", "Extraire les données du Projet Aurora"]
```

**Vérifié par un spike** (rustc 1.97.0, `serde` 1.0.229, `toml` 1.1.6, `unicode-width` 0.2.2, tous résolus par `cargo` le 7 octobre 2026) : les extraits ci-dessus (structure et textes FR/EN de M05, complétés dans le spike par M04) sont lus par `serde` + `toml` ; le spike vérifie la parité des clés structure / FR / EN, la parité du nombre d'objectifs, les budgets du §6.2 **en largeur d'affichage** (`unicode-width`) et signale un prérequis inconnu. Crates à retenir pour cette couche : `serde`, `toml`, `unicode-width` ; l'embarquement par `include_str!` suffit (`rust-embed` 8.13.0 si l'on veut lire le disque en développement ; `include_dir` 0.7.4 n'a plus été publié depuis juin 2024 : à éviter).

**Tests de contenu** (dans le crate moteur, exécutés par `cargo test`) :

| # | Test | Ce qu'il garantit |
|---|---|---|
| T1 | parité des clés FR/EN | aucune clé manquante ni en trop |
| T2 | parité des marqueurs `{handle}`, `{n}`… | mêmes variables dans les deux langues |
| T3 | budgets de longueur (§6.2) | aucune chaîne ne dépasse son budget |
| T4 | caractères interdits | pas d'ANSI, d'émoji, de contrôle, de `…`, d'espace insécable en entrée |
| T5 | lexique | aucune occurrence de « Halo », « Agent Smith », « Arasaka », « la toile » ; termes du glossaire (§2.7) |
| T6 | graphe de quêtes | acyclique ; prérequis, donneurs, systèmes, objets, contacts, fragments existent |
| T7 | accessibilité des objectifs | chaque objectif est réalisable au palier maximal de sa quête (système, commande, objet) |
| T8 | décisions et fins | toute combinaison D1 × D2 × D3 mène à exactement une fin ; chaque fin est atteignable |
| T9 | fragments et cinématiques | chaque fragment a une source ; chaque cinématique a un déclencheur |
| T10 | contacts | chaque contact donne une quête et a ses états ; un contact hostile, silencieux ou mort a toujours une cause tracée par un drapeau |
| T11 | épilogue | chaque état de contact a une ligne ; aucune ligne orpheline |
| T12 | épicène | aucun accord (« prêt », « content », « sûr(e) ») dans une réplique adressée au joueur (liste noire + revue) |
| T13 | indices | chaque objectif de quête principale a 2 indices, chaque objectif d'annexe 1 |
| T14 | aller-retour de sauvegarde | chaque clé `narr.*` se relit identique |
| T15 | pas d'impasse d'achat | tout objet exigé par une quête reste achetable, même si R4Z0R est hostile (le Courtier prend le relais) |

### 6.4 Ordre d'écriture

Le principe est une **tranche verticale d'abord** (D-10) : le lot L1 exerce tous les mécanismes (quêtes, courriers, fragments, cinématique, dialogues à variantes) sur deux chapitres, ce qui permet de tester le moteur et la voix avant d'écrire la suite. L'écriture ne dépend pas de la tactique des runs (la campagne se joue d'abord en `AutoResolve`, tâche A).

| Lot | Contenu | Quêtes | Chaînes / langue | Mots / langue (estim.) |
|---|---|---:|---:|---:|
| L0 | Cadrage : glossaire, clés, voix, budgets, gabarits (ce document), échantillons validés | — | — | — |
| L1 | Actes I : chapitres 1 et 2 (M01-M04, S01-S03, ECHO-7, R4Z0R, Data Miner) | 7 | 313 | 6 352 |
| L2 | Chapitre 3 : Projet Aurora (M05-M07, S04-S06, Phoenix, Neon Angel, Ghost Walker, Broker, AURA en signal) | 6 | 339 | 6 938 |
| L3 | Chapitre 4 : La Libération (M08-M09, S07-S10, S12-S13, Nexus Insider, AURA complète) | 8 | 257 | 5 348 |
| L4 | Chapitre 5 : Le Prix de la Vérité (M10-M12, S11, S14-S16, décision D1) | 7 | 215 | 4 812 |
| L5 | Chapitre 6 : Zénith (M13-M14, décision D3, fins, épilogue, scènes d'échec) | 2 | 140 | 4 684 |
| L6 | Transversal : ambiance (rumeurs, échelle de Notoriété, animations, programmes), documents chiffrés, interface, objets | — | 164 | 2 516 |
| L7 | Indices à 2 niveaux, passe d'accessibilité (lecteur d'écran, budgets TUI), parité FR/EN, recette de ton | — | (inclus ci-dessus) | — |

**Règle de livraison d'un lot** : (1) structure `data/world` complétée ; (2) textes FR ; (3) textes EN en parité le même jour ; (4) T1 à T15 verts ; (5) relecture de voix (2.5.4) par personnage ; (6) un instantané de partie jouable jusqu'à la fin du lot. **Dépendances** : tous les identifiants (quêtes, drapeaux, fragments, cinématiques) sont déjà fixés dans cette bible ; L2 écrit l'état « compromis » de Phoenix (B1) ; ses états libre, perdu, retourné et ceux de Neon Angel (silencieuse, libre) s'écrivent en L3 ; L5 (fins) ne s'écrit qu'après D1, D2, D3 figées.

**Gabarits.**
- *Quête* : `title` (3 mots), `title_short`, `desc` (une phrase, ≤ 110), `lore` (le pourquoi en 3 phrases : un détail concret, un enjeu, un doute), `loc`, objectifs (verbe à l'infinitif + objet + commande entre parenthèses), indices (1 piste, 2 solution), `debrief` (le donneur, avec son tic).
- *Sujet de conversation* : `q` (≤ 48 caractères, à l'infinitif ou en question courte), `a` (≤ 340, une idée, le tic du personnage, jamais d'exposition en bloc).
- *Courrier* : objet (≤ 48) ; corps en 4 temps : accroche dans la voix ; fait ; demande ou consigne chiffrée ; signature (+ un seul P.S. si le personnage en a l'habitude).
- *Cinématique* : un paragraphe = une image ; pas de dialogue de plus de 2 répliques d'affilée ; la dernière phrase est un geste du joueur ou un détail du monde.
- *Fragment* : en-tête du document (type, date, émetteur) puis 2 à 4 phrases ; le dernier mot ouvre une question.

### 6.5 Échantillons

Ces trois textes servent d'étalon de voix et ont été comptés en caractères contre les budgets du §6.2.

**Échantillon 1 : courrier de présentation de Neon Angel** (`mail.intro_angel`, envoyé à la fin de M05).

> **FR.** Objet : Ici Radio Veille
>
> Salut toi.
>
> Ici Neon Angel, voix de Radio Veille, trois heures du mat', secteur 7. Ta signature est passée sur nos relais : tu as ouvert l'œil du cyclone, et le cyclone a cligné.
>
> Les Éveillés te doivent un verre. On n'a pas de bar, alors on te doit une porte. Passe me voir : 'contact Neon Angel'.
>
> Garde l'œil ouvert,
> Neon Angel
>
> P.S. : Tu n'as pas de Halcyon. Garde ça pour toi, surtout devant les gens qui sourient.

> **EN.** Subject: This is Vigil Radio
>
> Hey you.
>
> This is Neon Angel, the voice of Vigil Radio. Three a.m., Sector 7. Your signature crossed our relays: you opened the eye of the storm, and the storm blinked.
>
> The Awake owe you a drink. We don't have a bar, so we owe you a door. Come see me: 'contact Neon Angel'.
>
> Stay awake,
> Neon Angel
>
> P.S.: You don't wear a Halcyon. Keep that to yourself, especially around the people who smile.

*Ce que l'échantillon fixe* : tutoiement chaleureux, argot de radio, signature `Garde l'œil ouvert` / `Stay awake`, indice du motif « sommeil », épicène (aucun accord avec le joueur), commande en clair entre apostrophes `[CANON]` (comme `contact Phoenix` dans le courrier de Phoenix).

**Échantillon 2 : bannière du chapitre 4 et ouverture de la cinématique CS05** (`chapter.4`, `cutscene.cs05.p01-p03`).

> **FR.** CHAPITRE 4 : LA LIBÉRATION
> Cette nuit-là, le secteur 7 s'est tu. Nexus appelle ça une Accalmie. Quelqu'un, dans le réseau, appelle ça un appel à l'aide.
>
> 02:57. Les enseignes du secteur 7 grésillent, comme toujours. Quelque part, un chien aboie, un klaxon répond, une dispute monte d'un étage.
>
> 03:00. Sur quarante mille nuques, un voyant passe du rouge au vert.
>
> 03:03. Le klaxon s'est arrêté. Le chien aussi. Dans la rue, une femme sourit à un mur. Votre deck n'affiche qu'une ligne : TOUT VA BIEN.

> **EN.** CHAPTER 4: THE LIBERATION
> That night, Sector 7 went quiet. Nexus calls it a Lull. Someone on the net calls it a cry for help.
>
> 02:57. The signs over Sector 7 sputter, as always. Somewhere a dog barks, a horn answers, an argument climbs a floor.
>
> 03:00. On forty thousand necks, an indicator light turns from red to green.
>
> 03:03. The horn has stopped. So has the dog. In the street, a woman smiles at a wall. Your deck shows a single line: ALL IS WELL.

*Ce que l'échantillon fixe* : narration au vouvoiement et au présent, une image par paragraphe, détail concret, aucune explication (« Halcyon » n'est pas nommé), la dernière phrase est un détail du monde (« TOUT VA BIEN », écho de la phrase de Radio Veille). La bannière reprend et prolonge `CHAPTER_4_TEXT` du canon (« Alliez-vous avec l'IA AURA… »), dont l'élan est repris en M09.

**Échantillon 3 : AURA et ECHO-7, fin de CS06** (voix, et indice avant l'aveu).

> **FR.**
> AURA : Je ne sais pas encore ce que vous appelez un matin. Pourriez-vous m'en décrire un ?
> ECHO-7 : D'après mes archives, c'est le moment où l'on regrette la veille.
> AURA : Merci. C'est la première définition qui me paraisse honnête.
> (Un silence. Le voyant du deck vacille.)
> AURA : Je te reconnais, toi. Pourquoi ne dis-tu rien ?

> **EN.**
> AURA: I do not yet know what you call a morning. Could you describe one to me?
> ECHO-7: According to my files, it's the moment you start regretting the night before.
> AURA: Thank you. That is the first definition that seems honest.
> (A silence. The deck's indicator light flickers.)
> AURA: I know you, though. Why aren't you saying anything?

*Ce que l'échantillon fixe* : AURA vouvoie le joueur mais **tutoie ECHO-7** en français (en anglais : elle n'emploie des contractions qu'avec lui) ; ECHO-7 dit « d'après mes archives » / « according to my files » (tic d'écho) ; didascalie entre parenthèses.

### 6.6 Alignement avec le cahier de refonte (tâche A)

| Sujet | Tâche A (`game-systems-audit.md`) | Cette bible | État |
|---|---|---|---|
| ECHO-7 | une IA, ancien prototype Aurora (décision 12) | écho numérique d'un humain mort, hébergé dans le deck | accord |
| Renommages | « Agent Smith » ; « Ghost Protocol » réservé aux outils | Agent Smith → Voss ; Phase 3 = Zénith, « Ghost Protocol » = ancien nom de code et nom de l'outil de R4Z0R | accord (précisé) |
| AURA | seule une partie est libre au niveau 5 ; la quête 7 libère le cœur | M09 héberge la **copie** ; le **Cœur** se décide en M13 | écart mineur (§4.2) |
| Fins | colonne vertébrale + 3 fins par drapeaux (décision 7) | 4 fins + variante, assemblées par modules ; CONFIER supprimable | écart → D-03 |
| Quêtes | 10 principales, 6-8 contrats | 14 principales (4 repliables), 9 + 7 contrats | écart → D-02 |
| Chapitres | 4 conservés | 6 (4 + 2) ; repli à 4 | écart mineur |
| Sites | 14 = 7 + 7 | 14 = 7 + 7, mêmes rôles, autres noms (§4.6) | accord |
| Saisie | numéros et noms, plus de codes à recopier (décision 10) | documents chiffrés sans saisie (D-07) | accord |
| Échec | pas de game over par défaut ; Hardcore | lignes courtes de run grillé + 3 scènes Hardcore | accord |
| Documents | ≈ 60 documents de butin de ≈ 70 mots | 24 fragments + 36 secondaires | accord |
| Chaleur | Trace (run) + Notoriété (Discret, Surveillé, Traqué, Chassé) | adoptée ; échelle d'origine INVISIBLE…MANHUNT retirée | accord |
| Progression | paliers liés aux quêtes ; plus d'XP par action | colonne « Règle A » ; colonne XP optionnelle | accord |
| Volume de texte | ≈ 29 k mots (22-33 k) | ≈ 28,7 k (cible) à 30,7 k (max) par langue | cohérent |
| Déblocage des contacts | par quêtes (§3.9), Neon Angel à la quête 8 | par quêtes, plus tôt (Neon Angel à M05) | écart (squelette modifiable) |

## 7. Vérifications et hypothèses

### 7.1 Vérifié en exécutant des commandes

| # | Commande ou méthode | Résultat |
|---|---|---|
| 1 | `git log --all --stat`, `git ls-tree -r af69f9b`, `git show af69f9b:<fichier>` | historique de 24 commits lu ; import d'origine `af69f9b` (`quest_system.c` 570 lignes, `contacts.c` 746, `neon_hack.c` 1 813, `advanced_hacking.c` 928) relu ligne à ligne pour les textes ; le tag `legacy-v2.087` n'existe pas (`git rev-parse` échoue) |
| 2 | `grep` de `display_lore_fragment`, `play_cutscene`, `display_chapter_intro`, `trigger_story_event`, `complete_quest` dans `af69f9b` | `play_cutscene` (l.500) et `display_chapter_intro` (l.536) sont définies ; `display_lore_fragment`, `trigger_story_event`, `complete_quest` sont **déclarées sans définition** ; aucun appelant dans `neon_hack.c` |
| 3 | `git log --all -S"<terme>"` pour « Agent Smith », « Arasaka », « TechDyne », « intro_nexus », « play_cutscene » | « intro_nexus » et `play_cutscene` retirés par `66b8b8b` ; « Arasaka » par `ff60708` ; TechDyne retiré des quêtes par `66b8b8b`, réapparu comme *TechDyne Research* dans `world.c` (`15d4964`) ; « Agent Smith » déplacé dans `cmd_hacking.c` par `4a126e4`, **toujours présent au HEAD** |
| 4 | analyse de `src/i18n/strings.def` par script Python | 436 entrées, 187 narratives, 3 262 / 2 997 mots FR / EN, 0 entrée vide, 0 espace insécable, 17 entrées avec symbole, longueurs des textes existants (§1.3, §6.2) |
| 5 | comptage `grep` des appels d'affichage dans les modules non portés | 291 (§1.3) |
| 6 | rejeu du déchiffrement de `cmd_hacking.c` en Python | `WKLV#LV#D#WHVW` → `THIS IS A TEST` ; `SKDVH#WZR#RI#SURMHFW#DXURUD#DSSURYHG` → `PHASE TWO OF PROJECT AURORA APPROVED` (chiffre de César -3, `#` = espace, lettres ASCII sans accent seulement) |
| 7 | script de cohérence du modèle de campagne (`check.py`, dossier de travail) | 30 quêtes (14 principales, 16 annexes dont 1 branche), 111 objectifs, 9 contacts, 16 systèmes en 14 sites, 24 fragments : 0 erreur ; toutes les vérifications décrites ci-après passent |
| 8 | mesure des échantillons et des budgets | §7.3 |
| 9 | lecture de `docs/design/game-systems-audit.md` (615 lignes) | alignement du §6.6 |
| 11 | spike Rust (`content-spike`, dossier de travail) : `cargo run` sur les extraits TOML du §6.3 | « OK : 2 quêtes, 24 budgets vérifiés, prérequis inconnus (attendu m03) » ; `rustc` 1.97.0 |
| 12 | `curl` sur l'API de crates.io | `toml` 1.1.6 (2026-09-10), `serde` 1.0.229, `unicode-width` 0.2.2, `rust-embed` 8.13.0, `include_dir` 0.7.4 (2024-06-17), `proptest` 1.11.0 |
| 10 | `git status` après travail | aucun fichier du dépôt modifié ; deux fichiers non suivis dans `docs/design/` (la tâche A et celui-ci) |

**Ce que `check.py` vérifie** : unicité des identifiants ; prérequis existants, acycliques, de palier et de chapitre croissants ; donneur existant et débloqué avant la quête (la quête de déblocage est un ancêtre) ; chaque système, commande, objet et contact référencé existe et est accessible au palier maximal de la quête (l'extraction de fichiers n'est pas modélisée, voir §7.2) ; chaque fichier cité existe dans son système ; chaque fragment a une source ; toute combinaison D1 × D2 × D3 mène à une fin et chaque fin est atteignable ; les 16 systèmes sont répartis en 14 sites ; le repli laisse 10 principales ; le palier proposé n'est jamais inférieur au palier de la règle A ; chaque contact donne au moins une quête.

### 7.2 Hypothèses et limites (non vérifié)

1. **Les nombres de mots et de chaînes sont des estimations** (hypothèses de longueur au §6.1), à ± 30 %. Aucun texte n'a été rédigé en dehors des échantillons.
2. **L'anglais** des échantillons et des exemples n'a pas été relu par un locuteur natif.
3. **`make test` n'a pas été relancé** (aucune compilation dans le dépôt) ; le brief le donne vert (26 suites, 105 e2e, 75 pty).
4. **L'équilibrage n'est pas simulé** : les récompenses sont relatives ; le « pas d'impasse d'XP » (`test_no_experience_dead_end`) et l'économie sont à vérifier par le harnais de la tâche A (`neon-sim`) ; le total de crédits promis (§5.1) est un ordre de grandeur.
5. **Les paliers, commandes et systèmes** sont ceux du moteur actuel ; la direction A les remplace par des programmes, des sites ouverts par l'histoire et des paliers par quêtes : la colonne « Règle A » et les prérequis restent valables, pas les commandes.
6. **Les noms nouveaux** (Elias Voss, Ren Aoyama, Kaito Ishida, Yuna Hayashi, Ilyas Benali, Teodor Wirth, Mizuki Arai, Hanae Kurokawa, Halcyon, Zénith, Les Éveillés, Radio Veille, AURORA-ZERO) n'ont pas été recherchés pour des conflits de marque (aucun accès à une recherche de marques pendant ce travail) ; risque estimé faible.
7. **Le rendu réel** des échantillons dans un TUI n'a pas été essayé ; seuls les budgets en caractères ont été comptés.
8. **Le compte de 291 appels d'affichage** compte les lignes contenant `printf(`, `print_colored_text(` ou `print_typing_effect(` ; la ROADMAP annonce ≈ 570 lignes d'affichage : méthodes de comptage différentes, écart non expliqué.
9. **Rapports `docs/legacy/RAPPORT_*.md` et `demo_*.sh`** : parcourus par `grep` (mots-clés narratifs), non lus en entier ; sans contenu narratif repéré.
10. **Cohérence des décisions avec la tâche A** : la tâche A est elle-même une proposition ; si elle change (moteur, nombre de sites, fins), reprendre le §6.6.
11. **Extraction de fichiers** : dans le moteur C actuel, extraire des fichiers exige `advhack`, `aihack` ou `temporalhack` selon le système, ou la Quantum Encryption Key pour les fichiers de niveau 1-2 (ARCHITECTURE.md, « Monde » et « Boutique » ; ROADMAP 3.2 : « aucune commande n'extrait avant le niveau 3 »). Les objectifs `EXTRACT` de M02 (optionnel), M04 (`financial_data.xlsx` est de niveau 3), S01 et S03 ne sont donc pas tous réalisables à leur palier dans ce moteur ; en direction A l'extraction est du butin ramassé en intrusion et la difficulté disparaît. À traiter côté moteur, pas côté texte.

### 7.3 Mesures sur les échantillons et les budgets

- courrier Neon Angel (FR) corps : 419 caractères (budget 600)
- courrier Neon Angel (EN) corps : 394 caractères (budget 600)
- bannière ch.4 FR : 125 caractères (budget 140)
- bannière ch.4 EN : 99 caractères (budget 140)
- paragraphes de CS05 : 5 mesurés, le plus long 139 caractères (budget 280)
- répliques de l'échantillon 3 : 8 mesurées, la plus longue 90 caractères (budget 340)
- textes existants au-delà des budgets du §6.2 : `QT_INTEL_OBJ_REP` FR 78 > 64, `QT_INTEL_OBJ_REP` EN 65 > 64, `QT_NEXUS_OBJ_ITEM` FR 114 > 64, `QT_NEXUS_OBJ_ITEM` EN 102 > 64

### 7.4 Livrable

Ce fichier : `docs/design/narrative-bible.md`. Les fichiers de travail (modèle de campagne, vérifications, comptes) sont dans le dossier temporaire de la session et ne font pas partie du dépôt.

## Annexe A. Textes d'origine à reprendre (verbatim, français seul)

Source : `af69f9b`, sauf mention ; typographie normalisée (guillemets français, retours à la ligne fondus, espace avant « : » supprimée comme à l'origine). Ces textes n'existent plus dans le HEAD (sauf le document « ultra-secret », encore en dur dans `cmd_hacking.c`). Ils sont à réécrire en parité FR/EN avec les corrections du §1.4 ; ils sont recopiés ici pour que l'auteur du texte ait la matière sous les yeux.

**A.1 Cinématique `intro_nexus`** (`quest_system.c:508-519`), reprise en CS02 :

> Dans les profondeurs de Neo-Tokyo, les serveurs de Nexus Corp bourdonnent d'une activité suspecte. Des flux de données cryptées circulent vers des destinations inconnues...
> Votre cyberdeck intercepte un fragment de transmission: « Phase 2 du Projet Aurora approuvée. Déploiement des implants de contrôle neural prévu pour le secteur 7... »
> Quelque chose de sinistre se trame. Il faut creuser plus profond...

**A.2 Cinématique `ai_liberation`** (`quest_system.c:520-530`), reprise en CS06 :

> Les serveurs de Nexus Corp s'illuminent soudainement. Une présence digitale se manifeste dans votre cyberdeck:
> « Hacker... Je suis AURA, l'IA du Projet Aurora. Nexus Corp me maintient prisonnière pour contrôler les esprits. Aidez-moi à me libérer, et ensemble nous pourrons exposer la vérité... »
> Une alliance inattendue vient de naître.

**A.3 Fiche du Shadow Broker** (`contacts.c:145-173`) : nom « Shadow Broker » ; identité « Identité multiple » ; description « Courtier d'informations le plus influent de Neo-Tokyo » ; spécialité « Achat/vente d'informations confidentielles » ; lieu « Dark Web - Serveurs anonymes » ; accueil « L'information a un prix. Que pouvez-vous m'offrir ? » ; trait « Calculateur et moralement neutre » ; histoire « Le Shadow Broker n'est pas une personne mais un réseau d'informateurs coordonné par une IA sophistiquée. Il possède des dossiers sur tous les habitants de Neo-Tokyo et vend ces informations au plus offrant, maintenant un équilibre délicat entre les factions en conflit. » ; niveau 4, réputation 100, disponibilité « Occupé », confiance 20 ; services : missions et informations.

**A.4 Document « ultra-secret »** (`cmd_hacking.c:519-524`, aussi `neon_hack.c:1294-1301`), déclenché par `quantumdecrypt CLASSIFIED` (fichier `project_ghost.dat`) : « DOCUMENT ULTRA-SECRET / PROJET GHOST PROTOCOL - PHASE 3 / Neo-Tokyo sera sous contrôle total d'ici 2088 / Coordinateur: Agent Smith / Budget: 50 000 000 crédits ». À réécrire en F07 avec Elias Voss et le nom de code « Ghost Protocol » donné comme ancien (§1.4, I3, I4).

**A.5 Descriptions des 8 méthodes avancées** (`advanced_hacking.c:36-113`) : Quantum « Utilise l'informatique quantique pour casser instantanément les encryptions » ; AI-assisted « L'IA analyse les patterns de sécurité et optimise l'attaque » ; Neural « Interface directe cerveau-machine pour un contrôle parfait » ; Stealth « Infiltration invisible, presque indétectable » ; Virus « Déploie des virus auto-réplicants et adaptatifs » ; Social « Manipulation psychologique des employés cibles » ; Zero-Day « Exploit inconnu des systèmes de défense » ; Ghost « Hack fantôme qui ne laisse aucune trace ».

**A.6 Les 14 animations « machine à écrire »** (`cmd_hacking.c:205-509`) : « Injection du payload... », « Création des hooks système... », « Masquage des traces... » (backdoor) ; « Envoi des paquets ICMP... » (traceroute) ; « Contournement antivirus... », « Injection du code malveillant... », « Activation du payload... » (virus) ; « Analyse des patterns de sécurité... », « Génération d'exploits adaptatifs... », « Exécution de l'attaque neuromorphe... » (IA) ; « Superposition des qubits... », « Algorithme de Shor en cours... », « Factorisation quantique... », « Effondrement de la fonction d'onde... » (quantique).

**A.7 Divers** : l'assistant « ECHO » (`advanced_hacking.c:337-346`) : nom « ECHO », personnalité « Analytique et méthodique », loyal, apprend ; les quatre techniques d'ingénierie sociale (l.647-651) : « Phishing par email », « Appel téléphonique (vishing) », « Infiltration physique », « Manipulation des réseaux sociaux » ; l'exemple de saisie quantique du jeu d'origine (`neon_hack.c:1644`) : `quantumdecrypt 'Q#X7#NEXUS#SECRET#DATA'` ; la conclusion d'origine (`neon_hack.c:1809-1810`) : « Merci d'avoir joué à Neon Hack ! / Gardez vos secrets... dans l'ombre. ».
