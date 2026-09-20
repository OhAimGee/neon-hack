# Plan de développement — de `refonte/v1` à la v1.0

> **Provenance.** Le plan d'origine a été perdu. Celui-ci est *reconstitué* (19 sept. 2026) d'après l'historique git, le code
> et la documentation : les messages de commit disent « Phase 0 » et « Phase 1 » ; les commentaires du code citent « Phase 3 »
> (port des modules d'origine, dont « 3.4 » pour l'économie) et « phase 5 » (équilibrage) ; `docs/ARCHITECTURE.md` cite
> « phase 8 » (matrice Windows/macOS). Ces numéros sont conservés pour que les références restent vraies ; les phases 2, 4, 6
> et 7 sont des étiquettes nouvelles.

**Objectif v1.0 — campagne complète** : un jeu publiable avec un début, un milieu et une fin (les 10 quêtes du Projet Aurora,
tous les contacts, un épilogue), entièrement traduit FR/EN, sans code d'origine non strict, équilibré par simulation, avec des
binaires Linux, macOS et Windows.

## Suivi

- [x] **Phase 0** — base saine (import du jeu d'origine, tag `legacy-v2.087`, licence, README honnête)
- [x] **Phase 1** — socle testé (entrées sûres, RNG, config, plateforme, terminal, i18n, CI)
- [x] **Phase 2** — couche de jeu unifiée : `GameState` unique, table de commandes, alerte, HUD, progression, monde unifié,
      menu + prologue + tutoriel + sauvegarde + réglages
- [ ] **Phase 3** — porter les modules d'origine
  - [x] 3.1 bus d'événements · [x] 3.2 moteur de quêtes · [x] 3.3 contacts et messages
  - [x] 3.4 boutique et économie · [ ] 3.5 hacking avancé et commandes, tout le code en règles strictes
  - [x] Hors lot : vitrine de la boutique lisible (2 colonnes) et saisie avec complétion par TAB
- [ ] **Phase 4** — contenu narratif (quêtes 5 à 9, épilogue, 5 contacts, fin)
- [ ] **Phase 5** — équilibrage par simulation
- [ ] **Phase 6** — confort de jeu
- [ ] **Phase 7** — durcissement
- [ ] **Phase 8** — multiplateforme et releases

## Constats de départ (vérifiés dans le code)

- ~~`update_quest_progress`, `check_quest_prerequisites`, `start_quest` et `unlock_contact` ne sont appelés par aucun code de
  jeu : aucun événement de gameplay n'alimente quêtes ni contacts. C'est le trou central.~~ **Résolu en 3.1 + 3.2** : bus
  d'événements, moteur de quêtes, puis (3.3) contacts refondus et pilotés par le bus.
- Seules **4 quêtes sur 10** sont écrites (tutoriel, `FIRST_INFILTRATION`, `GATHER_INTEL`, `NEXUS_DATA_BREACH`) : les
  5 à 9 et l'épilogue restent à écrire en 4.1 (elles sont verrouillées tant qu'elles n'ont pas d'objectif).
- ~~`use_item` (`shop.c`) est un **stub** : les 10 objets de la boutique n'ont aucun effet.~~ **Résolu en 3.4** : `nh_shop_buy`
  et les 10 effets réels, testés.
- ~~Contacts : 9 dans l'enum, **4 dialogues** écrits, 1 seul débloqué au départ, 5 sur un menu générique sans effet.~~ **Résolu
  en 3.3** pour les 4 contacts écrits (ECHO-7, R4Z0R, Phoenix, AURA, tous joignables) ; les 5 autres, sans fiche, restent
  verrouillés et sont à écrire en 4.2.
- Deux commandes de furtivité pour **deux états distincts** : `stealth` (cachée) bascule `gs->stealth_mode`, `stealthmode`
  bascule `gs->advanced.stealth.is_active`, qui n'est **pas sauvegardé**.
- ≈ 3 700 lignes d'origine non strictes (`contacts`, `advanced_hacking`, `cmd_hacking`, `quest_system`, `shop`, `cmd_world`,
  `cmd_advanced`, `game.c`), ≈ 570 lignes d'affichage (`printf`, `print_colored_text`, machine à écrire) en français en dur dans ces modules, couleurs en dur (`legacy_colors.h`),
  au moins 7 warnings (rien que dans `contacts.c`, `quest_system.c`, `advanced_hacking.c`). Le tutoriel fermait la quête d'origine
  à la main (`close_legacy_quest`, `tutorial.c`) : **supprimé en 3.2**, comme les warnings de `quest_system.c`.

## Phase 3 — Porter les modules d'origine

**Règle de port** : un module est *porté* quand il n'a plus de texte en dur (`strings.def`, FR+EN), plus de `legacy_colors.h`
(`nh_c`), ne lit l'entrée que par `core/io.h`, garde son état dans `GameState` **et dans `save.c`** (avec un test d'aller-retour),
est ajouté à `STRICT_OBJ` (Makefile) et a ses tests unitaires et e2e.

**Conception** : quêtes, contacts et objets deviennent des tables `static const` à clés `NhStr` (comme `k_commands` et `k_steps`)
au lieu de `strcpy` dans des `char[]` : le texte suit la langue, il n'y a rien à sauvegarder.

1. **3.1 Bus d'événements** — nouveau `src/game/events.[ch]` (strict) : `nh_event(gs, NhEvent, valeur)`, sur le modèle de
   `nh_tutorial_on_command` dans `nh_dispatch`. Émis depuis les points d'entrée *uniques* déjà en place : `nh_world_compromise`
   (`world.c` : cible piratée, fichiers, crédits), `nh_grant_xp` (`progression.c` : niveau atteint), achat en boutique,
   `contact_npc`, `nh_milestone_claim` (décryptage), réputation, discrétion (alerte).
   **Fait** : bus *différé* (`nh_event` enregistre, `nh_events_flush` livre à la fin de `nh_dispatch`, pour que les annonces
   suivent le résultat de la commande), abonnés « à niveau » qui relisent l'état du jeu. Émetteurs : commande exécutée,
   système compromis, fichiers extraits, niveau, réputation, achat, contact rencontré, jalon, quête terminée. La discrétion
   n'a pas d'événement propre : l'alerte se relit à chaque `NH_EV_COMMAND`. Détail dans `ARCHITECTURE.md`.
2. **3.2 Moteur de quêtes** (`quest_system.c`) : consomme les événements, active les quêtes par prérequis après chaque
   complétion ou montée de niveau, verse les récompenses **une seule fois** (XP via `nh_grant_xp`), journal `quests` FR/EN.
   `QUEST_INTRO_TUTORIAL` passe par le moteur, ce qui supprime `close_legacy_quest` (`tutorial.c`).
   **Fait** : `quest_system.[ch]` réécrit et strict, quatre quêtes en tables à clés `NhStr`, journal FR/EN, `test_events.c`,
   `test_quests.c` et un e2e « campagne » (Q1 → Q2 → Q3, alerte, anglais, rechargement). Écarts et constats :
   - **Impasse d'expérience corrigée** : un joueur de niveau 2 n'avait que 45 XP à gagner pour 60 requis au niveau 3 (partie
     bloquée). « Baptême du Feu » verse 25 XP ; `test_no_experience_dead_end` garde la porte.
   - **Objectifs d'origine remis d'aplomb** : « extraire 3 fichiers » au niveau 2 était impossible (aucune commande n'extrait avant
     le niveau 3), « TechDyne-Server » n'existe plus, 50 de réputation était hors d'atteinte (rien n'en donnait assez) : le
     *Street Cred Booster* de la boutique (+20) est donc implémenté dès maintenant, avec la réputation du tutoriel et des quêtes.
   - **Retiré du code d'origine, à réécrire en 4.1** : cinématiques (`play_cutscene`), fragments de lore, récompenses spéciales
     de quête (le jeu d'origine n'appelait rien de tout cela). Les introductions de chapitre sont portées (annoncées au démarrage de la première quête du
     chapitre, textes dans `strings.def`).
   - `laylow` « se faire accuser » retire 5 de réputation via `nh_grant_reputation` (bornée, émise sur le bus).
   - **Sauvegarde** : `quest.N.status`, `quest.N.obj.M` et `shop.bought` ; les anciens compteurs sont ignorés (version inchangée).
3. **3.3 Contacts et messages** (`contacts.c`, `cmd_world.c`) : `unlock_contact` piloté par niveau, réputation et quêtes via le
   bus ; les 4 dialogues existants portés (ECHO-7 cohérent avec le tutoriel, `nh_echo_say`) ; messages ajoutés en cours de
   partie **persistés** (aujourd'hui seul le masque « lu » des messages initiaux est sauvegardé). Remplace le petit abonné de
   `events.c` (`unlock_contacts`) ; **à traiter ici** : Phoenix (contact de « L'œil du Cyclone ») et AURA sont *hors ligne* et
   restent verrouillés, donc injoignables tant que ce lot n'est pas fait.
   **Fait** : `contacts.[ch]` réécrits et stricts, `test_contacts.c` (415 vérifications),
   e2e (liste, conversation, courrier persistant, Phoenix et AURA, anglais). Détail dans `ARCHITECTURE.md`, « Contacts et messages ».
   - **Tout est table** : `k_defs` (fiche, conditions de déblocage, sujets) et `k_mails` (modèles de courrier) à clés `NhStr` ;
     l'état ne garde que « débloqué » et « nombre de conversations » par contact, plus la boîte de réception (modèle + lu).
     La confiance et les services se déduisent (ils ne sont plus des champs qu'on pourrait laisser mentir).
   - **Déblocage par le bus** (`nh_contacts_on_event`) : R4Z0R niveau 2 + réputation 10 ; Phoenix niveau 4 + réputation 50 +
     « Réseaux d'Information » ; AURA niveau 5 + réputation 50 + « L'œil du Cyclone ». Annonce et courrier de présentation, une fois.
   - **Conversation en boucle** (elle se ferme par « Terminer », `0`, entrée vide ou fin d'entrée ; une saisie invalide
     redemande) ; le sujet « mission » parle de la quête *active de ce contact* et, pendant le tutoriel, redit la consigne.
     `nh_speak` (`ui/term`) met en forme les répliques de tous les personnages, ECHO-7 compris.
   - **Écarts assumés** : le sujet de R4Z0R « Proposer des données à vendre » affichait « +500 crédits » sans jamais les verser
     (ce n'aurait été qu'un farm de plus) : remplacé par « Demander comment gagner des crédits ». « Arasaka Corp » est devenu
     « MegaCorp Industries », la corporation du reste du jeu. La numérotation des contacts est celle de la *liste* (n-ième
     débloqué), là où l'ancien code en avait deux différentes. Les champs décoratifs (émoji, priorité, « chiffré ») sont
     abandonnés : plus aucun émoji dans le jeu porté. La fiche du Shadow Broker n'est pas portée (4.2).
   - **Sauvegarde** : `contact.N.flags` (bit 0 seulement), `contact.N.interactions`, `mail.count`, `mail.N.id`, `mail.N.read` ;
     les anciennes clés (`contacts.active`, bit « découvert », masque `inbox.read`) sont ignorées ou converties.
4. **3.4 Boutique et économie** (`shop.c`, `cmd_shop`) : implémenter les 10 effets (furtivité, réducteur d'alerte, virus,
   proxy/VPN, IA, puce quantique → `nh_world_sync_tools`, réputation, boost d'XP sous plafond anti-farm) ; revoir le
   `.credits = 5000` provisoire (`progression.c`) ; fermer les farms de crédits par la boutique et `advhack`.
   **Fait** : `shop.[ch]` réécrits et stricts (table `k_items`, `nh_shop_buy`), `test_shop.c` (306 vérifications), `test_economy.c`
   (3 194 vérifications, dont l'invariant anti-farm), e2e (dix effets à l'écran, dans la sauvegarde, après rechargement, refus).
   Détail dans `ARCHITECTURE.md`, « Boutique : achats, effets et économie ».
   - **Les 10 effets sont réels** et annoncés en FR/EN ; un achat qui ne changerait plus rien est refusé (`NH_BUY_MAXED`) au
     lieu d'encaisser. Le VPN devient permanent (`vpn_active`), le proxy un compteur de hacks (`proxy_hacks_left`), le Ghost
     Protocol une réserve consommée par `laylow` ; la clé de chiffrement ouvre les fichiers de niveau 1-2 ; le boost d'expérience
     double 3 gains déjà bornés (+30 au plus chacun).
   - **Économie sans cadeaux** : les 5 000 ¢, l'IA et l'ordinateur quantique offerts au niveau 5 disparaissent ; l'IA (500 ¢) et
     la puce quantique (800 ¢) s'achètent, et un gain de niveau annonce ce que R4Z0R met en vente.
   - **Farms fermés** : chaque source de crédits ou d'expérience est bornée par un état (`test_economy.c` le garantit sur un
     monde épuisé). Une idée de « vente de données » (jalon `NH_MS_DATA_SALE`) a été écartée : elle aurait payé deux fois.
   - **Laissé à la phase 5** : Ghost Protocol (80 ¢, −30) est dominé par « Se faire discret » (40 ¢, −25) ; deux récompenses
     uniques restent très élevées (10 000 ¢ et 2 000 ¢).
   - **Sauvegarde** : `player.key`, `player.xp_boost`, `alert.proxy_left` (l'ancien `alert.proxy` est ignoré).
5. **3.5 Hacking avancé et commandes** (`advanced_hacking.c`, `cmd_hacking.c`, `cmd_advanced.c`, `game.c`) : i18n, fusion des deux
   furtivités en un seul état **sauvegardé**, retrait de `legacy_colors.h` (`--no-color` complet), **tous** les fichiers dans les
   règles strictes (la distinction `STRICT_OBJ` disparaît), 0 warning.

*Fin de phase* : `make test` et `make asan` verts ; plus aucun littéral français accentué dans `src/game` hors `strings.def`
(un test le vérifie) ; sauvegarde relue à l'identique à chaque lot.

**Hors lot (fait après 3.2, à la demande du propriétaire du projet)** — deux défauts d'usage constatés en jouant :

- **Vitrine de la boutique** (`shop_view.[ch]`, strict) : l'ancien affichage (≈ 85 lignes) débordait de la zone de texte du HUD,
  le début du catalogue sortait par le haut et il fallait remonter dans l'historique du terminal (ce qui déplace aussi les
  barres). Le catalogue tient maintenant en entier, sur 2 colonnes dès 72 colonnes de terminal, en trois mises en page selon la place. Les
  textes de la boutique sont dans `strings.def` (FR+EN) et `ShopItem.description` a disparu. C'est la partie *affichage* de 3.4 ;
  les messages d'achat et les 10 effets ont suivi avec le lot 3.4.
- **Saisie avec complétion par TAB** (`ui/lineedit.[ch]` et `game/complete.[ch]`, stricts) : édition de la ligne, historique et
  TAB « comme bash » à l'invite de commandes ; les candidats sont les commandes disponibles maintenant, puis systèmes découverts,
  contacts débloqués ou numéros de messages selon `NhCommand.arg`. Sans terminal (tubes, tests, Windows), retour à `nh_read_line`.
  Ajouter une commande à argument = renseigner son `arg` dans `k_commands`. Les questions du menu, du prologue et de la boutique
  restent en lecture simple.
- Suites possibles, non planifiées : complétion des noms d'objets de la boutique, Ctrl+R (recherche dans l'historique),
  historique enregistré entre les sessions, saisie éditable sous Windows (`ReadConsoleInput`, phase 8).

## Phase 4 — Contenu narratif (campagne complète)

- **4.1** Les quêtes 5 à 9 (`UNDERGROUND_CONTACT`, `CORPORATE_SABOTAGE`, `AI_LIBERATION`, `SHADOW_BROKER`, `FINAL_SHOWDOWN`) et
  `EPILOGUE`, en tables de données (`k_quests`, voir 3.2) ; cinématiques et fragments de lore à écrire (les anciennes
  `play_cutscene` et `display_lore_fragment` ont disparu avec le code d'origine, elles étaient sans appelant ; le tag
  `legacy-v2.087` les garde). Les chapitres sont déjà annoncés par le moteur.
- **4.2** Les 5 contacts manquants (`SHADOW_BROKER`, `NEON_ANGEL`, `GHOST_WALKER`, `DATA_MINER`, `NEXUS_INSIDER`) : dialogues,
  services (`can_sell_items`, `can_give_missions`…), liés aux quêtes.
- **4.3** Fin et épilogue : écran de fin avec bilan, game over cohérent avec l'histoire.
- *Vérification* : un e2e « campagne complète » scripté (graine fixe) qui atteint l'épilogue, avec sauvegarde et rechargement à
  chaque chapitre.

## Phase 5 — Équilibrage par simulation

- Harnais sans affichage (`tests/sim/`) qui joue des stratégies scriptées via `nh_dispatch` et `nh_io_set_input` (comme
  `test_tutorial.c` et `nh_feed.h`), graines fixes : nombre de commandes par niveau, courbe de crédits, taux de game over, part
  de chaque source de revenus, quêtes terminables.
- À régler : `k_rewards` (`progression.c`), `k_formulas` et gains des 7 systèmes (`world.c`), `k_reductions` (`alert.c`), prix de
  la boutique, récompenses de quêtes. **Cibles chiffrées à fixer avec le propriétaire du projet** (durée de campagne, taux de
  game over) avant de régler quoi que ce soit.
- Invariants testés : aucune boucle de commandes ne fait monter crédits ou XP sans plafond.
- Points déjà repérés : passer le tutoriel coûte 100 ¢ et 10 de réputation, si bien que « Réseaux d'Information » (50 de
  réputation) exige alors deux *Street Cred Booster* au lieu d'un ; l'XP disponible à chaque niveau est juste (140 XP exactement
  pour le niveau 4 à la fin de « Réseaux d'Information ») ; les récompenses de quêtes (`k_quests`) sont provisoires.

## Phase 6 — Confort de jeu

Plusieurs emplacements de sauvegarde (et suppression) ; `--new` demande confirmation ; prologue passable (≈ 9 s aujourd'hui) ;
écran d'accueil et ASCII art traduits ; message « partie chargée » ; `__pycache__` dans `.gitignore` et `.pyc` retiré du suivi.

## Phase 7 — Durcissement

- Test « singe » : séquences de commandes aléatoires à graine fixe, avec invariants (niveau, crédits et alerte bornés,
  sauvegarde→chargement→sauvegarde identique à chaque pas, ni crash ni boucle, EOF partout), sous ASan/UBSan.
- Fuzz léger des chargeurs (`nh_kv_parse`, `nh_save_from_text`, `nh_settings_from_text`) ; mesure de couverture (gcov) des
  modules stricts.

## Phase 8 — Multiplateforme et releases

- **CI macOS et Windows (MSYS2/MinGW)** : compiler enfin les branches Windows jamais compilées (`platform.c`, `storage.c`
  `MoveFileEx`/`_mkdir`, `term.c` `nh_term_size`) ; rendre `tests/e2e/run.sh` portable (`timeout` absent sur macOS) ; pty réservé
  à Linux/macOS. Décider Makefile ou CMake à ce moment. Dossier de données macOS : `~/Library/Application Support/neon-hack`.
- **Releases** : workflow sur tag `v*` → binaires Linux/macOS/Windows + sha256, `NH_VERSION` tiré de `git describe`, CHANGELOG,
  section « Installation » du README.

## Ordre et dépendances

`3.1 → 3.2 → (3.3 ∥ 3.4) → 3.5 → 4 → 5 → 6 → 7 → 8`.

**Exception recommandée** : lancer la CI macOS/Windows de la phase 8 dès la fin de la phase 3. C'est le plus gros risque
technique (le code Windows n'a jamais été compilé) et il vaut mieux le découvrir tôt.

## Prochain lot : 3.5 Hacking avancé et commandes

1. **`advanced_hacking.c`, `cmd_hacking.c`, `cmd_advanced.c`, `game.c`** : textes dans `strings.def` (FR+EN), retrait de
   `legacy_colors.h` (`--no-color` complet), lecture par `core/io.h` seulement.
2. **Une seule furtivité**, sauvegardée : fusion de `stealth` (`gs->stealth_mode`) et `stealthmode` (`gs->advanced.stealth.is_active`,
   aujourd'hui perdu à chaque partie).
3. **Tout le code en règles strictes** : la distinction `STRICT_OBJ` disparaît, 0 warning (il reste `advanced_hacking.c:737`).
4. *Fin de phase 3* : un test vérifie qu'il ne reste aucun littéral français accentué dans `src/game` hors `strings.def`.
5. Tests et documentation, comme pour les lots précédents ; la CI macOS/Windows peut démarrer dès la fin de la phase 3.

## Risques

- **Volume d'écriture** (≈ 570 lignes d'affichage à traduire, ≈ 1 000 caractères de lore par quête, 6 quêtes à écrire) : porter par lots, chacun livrable.
- **Régression de sauvegarde** : toute donnée persistante nouvelle passe par `save.c` avec un test d'aller-retour ;
  `NH_SAVE_VERSION` ne monte que pour un changement incompatible (clé absente = valeur par défaut).
- **Windows jamais compilé** : d'où la CI précoce.
- **Équilibrage subjectif** : cibles chiffrées validées avant de régler.

## Vérification transversale

`make test` et `make asan` verts, CI verte sur tous les compilateurs et systèmes de la matrice, et, pour chaque phase, le
critère de fin indiqué ci-dessus.
