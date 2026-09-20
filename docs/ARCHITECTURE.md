# Architecture

Neon Hack est en cours de refonte selon une approche **« strangler »** : le nouveau
socle se construit à côté du code d'origine, le jeu reste jouable à chaque étape,
et les modules d'origine sont portés un par un avant d'être supprimés.

Les numéros de phase cités dans le code et dans cette documentation (« phase 3 »,
« phase 3.4 », « phase 5 », « phase 8 »…) renvoient à la [feuille de route](ROADMAP.md).

## Organisation actuelle

```
src/main.c         point d'entrée : options, réglages, création du GameState, menu, boucle
src/game/          logique de jeu (GameState unique, plus aucune variable globale d'état)
  game.[ch]          GameState, init_game, boucle de jeu, progression (gain_experience)
  progression.[ch]   courbe d'expérience, niveaux 1-6, déblocages, récompenses uniques ; nh_grant_xp() est le SEUL point d'entrée
  world.[ch]         le monde : graphe unique de systèmes (relais, découverte), état persistant, chances de succès, récompense unique
  events.[ch]        bus d'événements différé : nh_event() enregistre, nh_events_flush() (fin de nh_dispatch) livre aux abonnés
  quest_system.[ch]  quêtes : tables static const, objectifs mesurés sur l'état du jeu, récompenses uniques, journal `quests`
  alert.[ch]         alerte unique 0-100 : hausse, refroidissement, seuils, méthodes de réduction, affichage
  commands.[ch]      table de commandes, dispatch, aide, commandes système (help/status/save/quit/clear)
  menu.[ch]          menu de lancement (continuer, nouvelle partie, langue, options) et nh_start_new_game
  intro.[ch]         prologue d'une nouvelle partie : récit, ECHO-7, choix du nom, tutoriel ou non
  tutorial.[ch]      tutoriel : machine à états branchée sur nh_dispatch, mission « Premiers Pas dans l'Ombre »
  save.[ch]          sauvegarde et chargement de la partie (format texte clé=valeur, versionné)
  cmd_hacking.c      commandes de hacking classiques (scan, bruteforce, decrypt, backdoor…)
  complete.[ch]      ce que TAB propose à l'invite : commandes disponibles maintenant, puis l'argument selon NhCommand.arg
  shop_view.[ch]     vitrine de la boutique : composition pure (largeur, budget de lignes), 2 colonnes quand l'écran le permet
  shop.[ch]          boutique : table des 10 objets, achat `nh_shop_buy`, leurs effets réels
  contacts.[ch]      contacts et boîte de réception : tables, déblocage par le bus, conversations à menu, courriers
  cmd_world.c        `shop` et `laylow` (`quests` est dans quest_system.c, `contacts`/`contact`/`messages`/`read` dans contacts.c)
  cmd_advanced.c     hacking avancé (advhack, aiassist, neuralsync, temporalhack…)
  advanced_hacking   dernier module d'origine non porté (phase 3.5)
src/core/          socle neuf, testé, sans état de jeu
  platform.[ch]      pauses, mode rapide, console (Windows : UTF-8 + ANSI), détection du terminal
  io.[ch]            lecture de lignes et d'entiers sûre, EOF géré ; nh_io_uses_stdin() dit si l'entrée est la vraie
  parse.[ch]         découpe « commande argument », comparaison et préfixe sans casse
  utf8.[ch]          coupe propre d'une chaîne UTF-8 tronquée
  rng.[ch]           générateur PCG32 reproductible (graine, tirages sans biais)
  config.[ch]        options de ligne de commande, variables d'environnement, application des réglages enregistrés
  kv.[ch]            format texte « clé=valeur » : analyse tolérante, entiers stricts bornés, écrivain
  storage.[ch]       dossier de données, lecture bornée, écriture atomique (temporaire + fsync + renommage)
  settings.[ch]      réglages du joueur (langue, couleurs, animations, HUD) dans settings.cfg
src/ui/term.[ch]   couleurs ANSI, largeur d'affichage UTF-8, remplissage de colonnes, jauge, troncature, répliques de personnage (nh_speak)
src/ui/hud.[ch]    interface fixe : barres haut/bas + zone de texte défilante (région de défilement ANSI)
src/ui/lineedit.[ch]   saisie d'une ligne : édition, historique, complétion par TAB (cœur pur + mode brut termios)
src/i18n/          textes français/anglais (strings.def, i18n.[ch])
tests/unit/        tests unitaires (un exécutable par fichier ; test_commands.c teste la couche jeu ; nh_feed.h fournit la saisie,
                   nh_tmp.h des dossiers temporaires : les tests ne touchent jamais aux vraies sauvegardes)
tests/e2e/run.sh   tests de bout en bout du jeu compilé (sorties redirigées, dossier de données jetable par exécution)
tests/e2e/pty_hud.py   tests de l'interface fixe, du menu, du tutoriel, de la boutique et de la saisie (TAB, édition, historique)
                   dans un vrai pseudo-terminal + mini-émulateur d'écran
```

Le code neuf (`src/core`, `src/ui`, `src/i18n`, `src/main.c`, `src/game/commands.c`, `src/game/alert.c`, `src/game/progression.c`, `src/game/world.c`,
`src/game/save.c`, `src/game/tutorial.c`, `src/game/intro.c`, `src/game/menu.c`, `src/game/events.c`, `src/game/quest_system.c`,
`src/game/shop_view.c`, `src/game/complete.c`, `src/game/shop.c`, `src/game/contacts.c`)
est compilé avec `-Wpedantic -Wshadow -Wconversion -Werror`. Le reste de `src/game/`
(code d'origine déplacé) ne l'est pas : il porte encore ses avertissements et sera
remplacé, pas corrigé.

## État de jeu et commandes

- **`GameState`** (`src/game/game.h`) regroupe tout : joueur, nœuds réseau, boutique,
  alerte, quêtes, contacts, hacking avancé. Il est alloué par `main()` et passé à
  chaque handler ; il n'y a plus de variable globale d'état. Deux `GameState` peuvent
  donc coexister (les tests unitaires en créent un par cas).
- **Table de commandes** (`k_commands[]` dans `commands.c`) : *une seule source de
  vérité* pour le nom, l'alias, la catégorie, la condition de déblocage (drapeau
  `CMD_*` ou niveau minimum), l'aide (clé i18n) et le handler
  `bool fn(GameState *, const char *arg)`. Le dispatch, l'aide et les messages de
  verrouillage en sortent tous : une commande affichée dans `help` est exécutable par
  construction, et `test_commands.c` le vérifie.
- **`nh_dispatch()`** renvoie `NH_DISPATCH_OK / FAILED / EMPTY / UNKNOWN / LOCKED` ;
  la casse et les espaces autour de la ligne sont ignorés.
- Ajouter une commande = une ligne dans la table + une clé `HELP_<nom>` dans
  `strings.def` (sans elle, le projet ne compile pas). Le dernier champ, `arg` (`NhArgKind` :
  `NH_ARG_NONE`, `_SYSTEM`, `_CONTACT`, `_MESSAGE`), dit ce que TAB doit proposer après le nom de la
  commande (voir « Saisie de ligne et complétion »).
- **Alerte** (`alert.c`) : une seule valeur, `gs->alert.level`, de 0 à 100 (l'ancien code en avait
  trois qui s'écrasaient). Les fonctions `nh_alert_add/reduce/decay` ne font ni affichage ni tirage
  aléatoire ; `nh_alert_raise()` ajoute *et* annonce. Le temps passe à chaque action de hacking
  (`nh_dispatch`) : l'alerte se refroidit de 1 (+1 VPN, +1 proxy) avant que l'action n'ajoute la
  sienne ; consulter l'aide, le statut, la boutique ou les quêtes ne la fait pas baisser.
  Seuils : 30 attention, 50 niveau élevé, 70 danger, 80 boutique fermée, 100 game over. Malus sur
  les chances de réussite : -5 dès 30, -10 dès 50, -20 dès 80 (bruteforce, backdoor, virus, IA).
  Valeurs provisoires : l'équilibrage se fait en phase 5.
- **Progression** (`progression.c`) : toute expérience passe par `nh_grant_xp()`, qui applique autant
  de montées de niveau que la courbe le permet (cumulé : 15 / 60 / 140 / 260 / 420 pour les niveaux
  2 à 6) avec leurs déblocages, décrits dans la table `k_rewards`. Les commandes « nouvellement
  disponibles » annoncées sont déduites de la table de commandes avant/après (jamais d'écart avec
  `help`). Le hacking avancé ne verse plus rien lui-même : crédits et expérience d'un système sont
  versés par `nh_world_compromise()` (voir « Monde »).
  **Règle anti-farm** : une source d'expérience ou de crédits doit être limitée par un état — un
  budget (scans : 5+4+3+2+1), un drapeau sur le nœud (traceroute, système déjà compromis) ou un jalon
  à usage unique (`nh_milestone_claim` : message de test, premier décryptage quantique, document
  ultra-secret). Toute nouvelle récompense répétable doit suivre cette règle.
- **Monde** (`world.c`) : un seul graphe, `gs->nodes[]`, construit depuis une table `static const`
  (7 systèmes ; l'ancien code en avait 3 pour les commandes classiques et 5 cibles séparées pour `advhack`).

  ```
  localhost ─┬─ corp-server-01 ─┬─ nexus-mainframe ─ gov-database
             │                  └─ research-lab
             └─ underground-market ─ banking-network
  ```

  - **Découverte** : `scan` révèle les systèmes dont le niveau minimum est atteint (`nh_world_discover`) ;
    seul `localhost` est connu au départ. Une commande ne vise qu'un système découvert.
  - **Relais** : un système n'est atteignable que si son relais est compromis (`nh_world_reachable`).
    `traceroute` (passif) et `analyzedefenses` ne demandent pas de route ; `exploit` (système déjà
    tracé) et `temporalhack` la contournent. `nh_world_resolve()` fait la recherche *et* explique
    l'échec : toutes les commandes passent par là (plus de boucle de recherche recopiée).
  - **État persistant** par système : découvert, compromis, backdoor, virus, tracé, accès internes
    (`socialeng`), fichiers extraits. Compromettre (`nh_world_compromise`) verse les crédits et
    l'expérience du système **une seule fois** ; `deep` (IA, `advhack`, `temporalhack`) extrait aussi
    les fichiers. Sur un système déjà compromis, `aihack` n'extrait que les fichiers restants.
  - **Chances de succès** : une formule par méthode dans `k_formulas` (`nh_world_chance`), bornée à
    5-95 % ; les malus d'alerte, le mode furtif, la discrétion du virus passent en `bonus`. Les accès
    internes ajoutent 15 points sur tout le système. Les méthodes de `advhack` gardent leur propre
    formule (`calculate_hack_success_rate`) mais reçoivent le même `bonus`.
  - **Outils avancés** : leur état actif est *déduit* de ce que le joueur possède
    (`nh_world_sync_tools`) — l'ancien code n'avait aucune commande pour les activer, donc aucune
    méthode d'`advhack` (toutes exigent un outil) n'était jamais utilisable.
  - Les 5 systèmes qui ont un profil de défense avancé (`AdvancedTarget`) sont reliés par
    `nh_world_adv_target()` ; les autres (`localhost`, `corp-server-01`) n'acceptent que les commandes
    classiques.

## Événements et quêtes

- **Bus d'événements** (`events.[ch]`) : ce qui vient de se passer dans le monde est dit *une fois*, à l'endroit où cela
  arrive, par `nh_event(gs, type, valeur)` : système compromis (`nh_world_compromise`), fichiers extraits
  (`nh_world_extract`), niveau gagné (`level_up`), réputation (`nh_grant_reputation`), achat (`nh_shop_buy`), conversation
  (`nh_contact_talk`), jalon (`nh_milestone_claim`), quête terminée, et `NH_EV_COMMAND` à la fin de chaque commande exécutée
  (le temps a passé : l'alerte a bougé). L'émetteur ne sait pas qui écoute.
  - **Différé.** `nh_event()` ne fait qu'enregistrer, dans une file circulaire de 32 ; `nh_events_flush()`, appelée par
    `nh_dispatch()` une fois la commande terminée (et l'étape du tutoriel jouée), livre dans l'ordre. Les annonces
    (« NOUVELLE QUÊTE », « Objectif accompli ») s'affichent donc *après* le résultat de la commande et non au milieu de
    ses lignes ; et un abonné peut émettre à son tour — une quête terminée donne de l'expérience, donc un niveau, donc une
    autre quête — sans jamais être rappelé pendant qu'il s'exécute : ces événements sont livrés par la même boucle, pas
    par récursion. Garde-fous : file pleine, l'événement est perdu mais compté (`dropped`) ; livraison coupée à 256. Une
    partie finie (`quit`, game over) vide la file : pas d'annonce sur un écran de fin.
  - **Les abonnés relisent l'état du jeu** au lieu de compter les événements ; en perdre un ne fausse donc rien. Abonnés
    actuels, dans cet ordre : les contacts (`nh_contacts_on_event`, voir « Contacts et messages ») puis le moteur de
    quêtes — un contact débloqué par une quête terminée l'est donc avant que les quêtes suivantes ne réagissent.
  - L'état du bus vit dans `GameState.events` mais n'est **jamais sauvegardé** : il est vide entre deux commandes.
- **Quêtes** (`quest_system.[ch]`) : une table `static const` (`k_quests`) décrit chaque quête — textes (clés de
  `strings.def`), contact, niveau requis, prérequis, chapitre, objectifs, récompenses — et l'état par quête se réduit à
  `QuestState {status, progress[]}`. Cycle : VERROUILLÉE → ACTIVE dès que le niveau et les prérequis le permettent →
  TERMINÉE quand **tous** les objectifs sont accomplis en même temps. Une quête sans objectif (les 5 à 9, pas encore
  écrites) reste verrouillée. Les statuts DISPONIBLE et ÉCHOUÉE sont réservés (quêtes annexes) et jamais produits ; les
  valeurs sont écrites dans les sauvegardes, on ne les réordonne pas.
  - **Objectifs « à niveau »** (`measure()`) : chaque type se lit dans l'état du jeu (niveau, systèmes compromis, fichiers
    extraits, masque d'achats `shop.bought`, `interactions_count` d'un contact, jalons, réputation, alerte). L'avancement ne
    recule jamais — la réputation gagnée est acquise — sauf `NH_OBJ_KEEP_ALERT_BELOW`, une *condition* : vraie ou fausse à
    l'instant où l'on conclut. Un joueur qui avait déjà tout accompli quand la quête démarre (ancienne sauvegarde, ordre
    libre du joueur) la termine d'un coup, sans annonce d'objectif.
  - **Récompenses une seule fois** : `nh_quest_complete()` passe le statut à TERMINÉE *avant* de payer (crédits, puis
    `nh_grant_reputation` et `nh_grant_xp`) ; rien de ce que le paiement déclenche ne peut donc la payer une seconde fois.
    Refusée si la quête n'est pas ACTIVE. Avec `reward = false`, la quête est close en silence (tutoriel passé).
  - **Pas d'impasse d'expérience** : à chaque niveau, ce qu'on peut encore gagner sans le niveau suivant (scans, systèmes
    révélés, quêtes ouvertes) doit suffire à l'atteindre. À l'origine, un joueur de niveau 2 n'avait que 45 XP à gagner
    pour 60 requis : « Baptême du Feu » en verse 25 pour cela. `test_no_experience_dead_end` (`test_quests.c`) le
    vérifie : toute nouvelle quête ou tout changement de la courbe repasse par lui.
  - **Tutoriel** : `QUEST_INTRO_TUTORIAL` est la première quête, avec un objectif *manuel* que `tutorial.c` accomplit
    (`nh_quest_complete`). Tant qu'il est actif, `quests` montre la mission d'ECHO-7 au lieu du journal.
  - **Ajouter une quête** : une entrée dans `k_quests` et ses clés `QT_*` dans `strings.def` ; `test_quests.c` vérifie les
    tables (traductions, prérequis acycliques, arguments d'objectifs), l'absence d'impasse et, de bout en bout, la
    campagne des quatre premières quêtes. `tests/e2e/run.sh` la rejoue avec le vrai binaire à partir de sauvegardes
    fabriquées (`craft_save`) pour ne pas dépendre du hasard des piratages.

## Boutique : la vitrine

L'ancien affichage de la boutique faisait ≈ 85 lignes (une carte de 7 lignes par objet) dans une zone de texte de
22 lignes : le début du catalogue sortait par le haut, et le joueur devait remonter dans l'historique du terminal, ce
qui emporte aussi les barres du HUD (elles font partie de l'écran normal, seul le texte défile dans la région). Le
catalogue doit maintenant tenir **entièrement** dans la zone de texte.

- **`shop_view.c` est pur** : `nh_shop_compose(out, size, shop, credits, level, cols, max_lines)` écrit dans un tampon et ne
  lit rien sur le terminal ; `nh_shop_show()` n'est que la colle (largeur et hauteur du terminal, moins les deux barres
  quand le HUD est actif, moins la ligne de la question posée juste après). Sortie redirigée : 80 colonnes, sans limite.
- **Trois mises en page**, la plus confortable qui tient étant retenue : cartes aérées (bandeau, mot de bienvenue, une ligne
  vide entre les rangées : 28 lignes pour 10 objets sur deux colonnes), cartes serrées (20 lignes), puis liste d'une ligne par
  objet (13 lignes). Une carte fait 3 lignes. À partir de 71 colonnes utiles (largeur plafonnée à 100), les objets se
  répartissent sur **deux colonnes**, dans l'ordre de lecture par colonne (1 à 5, puis 6 à 10). À 80×24 avec le HUD (21 lignes
  disponibles) : deux colonnes de cartes serrées.
- Une case : `[NN] nom … prix` / résumé / `type · état`. L'état suit l'ordre de `nh_shop_buy()` (épuisé, niveau, crédits) pour
  que l'écran n'annonce jamais ce que l'achat refuserait autrement. Aucune ligne ne dépasse `cols - 1` colonnes (on n'écrit
  pas dans la dernière colonne) ; les textes trop longs sont coupés par `…` sans casser un caractère UTF-8.
- Les textes (titre, états, 10 résumés) sont dans `strings.def` (`SHOP_*`). `ShopItem.description` (texte français en dur)
  a disparu. Les achats et leurs effets sont décrits dans la section suivante.
- `test_shop_view.c` balaie les largeurs et hauteurs (chaque ligne tient, chaque objet est présent, FR et EN, couleurs ou
  non) ; `pty_hud.py` vérifie sur un vrai pseudo-terminal que tous les objets sont à l'écran et que les barres n'ont pas bougé.

## Boutique : achats, effets et économie

- **`shop.c`** (strict) : le catalogue est une table `static const` (`k_items` : nom de marque, prix, niveau, unique ou
  consommable) ; les noms sont identiques en français et en anglais, les résumés affichés sont dans `shop_view.c`.
  `test_shop.c` vérifie que les nombres cités dans ces résumés sont ceux du code (un résumé ne peut pas mentir).
- **`nh_shop_buy(gs, type)`** est le seul chemin d'achat (la commande `shop` et la conversation avec R4Z0R y passent). Il
  vérifie dans l'ordre : objet valide, pas épuisé, niveau, crédits, plafond (`NH_BUY_MAXED` : on refuse d'encaisser pour un
  objet qui ne changerait plus rien). Un refus est expliqué à l'écran et ne débite rien. Un achat débite, marque `shop.bought`,
  applique l'effet, annonce tout (FR/EN) et émet `NH_EV_ITEM_BOUGHT`. Un objet *unique* devient épuisé ; un *consommable* se
  rachète tant que sa réserve n'est pas pleine.

  | Objet (prix · niveau) | Effet réel |
  |---|---|
  | Stealth Module v2.0 (150 · 2, unique) | +2 de furtivité, plafonnée à 10 (`status` affiche `n/10`) |
  | Ghost Protocol (80 · 1, conso.) | +1 en réserve (max 5) ; le menu de `laylow` en consomme un pour −30 d'alerte |
  | Malware Arsenal (120 · 4, unique) | bibliothèque de virus complète (`uploadvirus`) |
  | Proxy Chain Pro (100 · 2, conso.) | 5 piratages dont la hausse d'alerte est réduite d'un tiers (arrondi au-dessus, jamais nulle) ; réserve max 15 |
  | Quantum Encryption Key (200 · 3, unique) | ouvre les fichiers chiffrés de niveau 1 à 2 (`NH_KEY_DECRYPT_LEVEL`), y compris ceux des systèmes déjà compromis |
  | Neural Assistant v3.1 (500 · 4, unique) | l'IA : `aihack` et `aiassist` (via `nh_world_sync_tools`) |
  | Quantum Processing Chip (800 · 5, unique) | l'ordinateur quantique : `quantumdecrypt` (via `nh_world_sync_tools`) |
  | Street Cred Booster (75 · 1, conso.) | +20 de réputation (`nh_grant_reputation`, donc bornée et émise sur le bus) |
  | Neural Accelerator (90 · 2, conso.) | 3 prochains gains d'expérience doublés, le bonus étant plafonné à +30 par gain |
  | Dark Web VPN (60 · 1, unique) | abonnement : le refroidissement gagne 1 point de plus à chaque action de hacking |

- **Alerte** : `AlertSystem.vpn_active` (permanent) et `proxy_hacks_left` (compteur, décrémenté par `nh_alert_end_action`
  après une commande de hacking classique ou avancée) ; l'ancien booléen `proxy_active` a disparu. Les méthodes de `laylow`
  « VPN » et « proxy » n'installent plus rien de durable : elles baissent l'alerte une fois, comme les autres.
- **Économie : plus rien n'est offert.** Le niveau 5 ne donnait plus 5 000 ¢, l'IA et l'ordinateur quantique d'office : ils
  s'achètent (500 ¢ au niveau 4, 800 ¢ au niveau 5) et le gain de niveau annonce ce que R4Z0R met en vente à ce niveau.
- **Règle anti-farm, appliquée à tout ce que la boutique touche** : une source d'expérience ou de crédits est bornée par un
  état, et le boost d'expérience double un gain *déjà borné* (jamais plus de +30 par gain, 3 gains par achat) au lieu
  d'en créer un. Les récompenses de quête passent par `nh_grant_xp_flat` : un boost ne double pas une récompense unique.
  `test_economy.c` en fait un invariant : sur une partie où tout a été pris (systèmes piratés, fichiers ouverts, jalons
  obtenus), chaque commande répétée des milliers de fois ne rapporte plus ni crédit, ni expérience, ni réputation ; il
  échoue dès qu'un farm réapparaît (vérifié en ouvrant volontairement une source).
- **Valeurs provisoires** : prix, plafonds et durées se règlent en phase 5. Déséquilibres connus laissés à cette phase :
  Ghost Protocol (80 ¢, −30) est dominé par le « Se faire discret » de `laylow` (40 ¢, −25), et deux récompenses uniques
  restent très élevées (10 000 ¢ et 2 000 ¢).

## Contacts et messages

- **Contenu en tables** (`contacts.c`, strict) : `k_defs[CONTACT_COUNT]` décrit chaque contact (nom propre, couleur,
  description, lieu, dossier, confiance de départ, conditions de déblocage, sujets de conversation) et `k_mails[]` chaque
  modèle de courrier (expéditeur, sujet, texte, contact dont il marque le déblocage). Tous les textes sont des clés de
  `strings.def` (`CT_*`, `MAIL_*`) : ils suivent la langue et ne sont jamais sauvegardés. Ajouter un contact ou un courrier =
  une entrée de table et ses clés ; `test_contacts.c` vérifie les tables (traductions non vides, dernier sujet = « Terminer »).
- **État minimal** (`ContactSystem`) : par contact `{is_unlocked, interactions_count}` ; la boîte de réception est une liste
  ordonnée de `{modèle, lu}` (32 au plus, un modèle n'arrive qu'une fois). La **confiance est déduite** (départ de la fiche
  + 3 par conversation, plafonnée à 100) et donne la *relation* affichée (inconnu < 20 ≤ neutre < 50 ≤ amical < 80 ≤ de
  confiance) ; les services d'un contact se déduisent de ses sujets, la liste ne peut donc pas promettre plus que la
  conversation ne tient.
- **Déblocage** : `nh_contacts_on_event` est un abonné du bus (comme le moteur de quêtes) et relit l'état du jeu : niveau,
  réputation ET quête terminée (s'il y en a une) demandés par la fiche. Un contact débloqué le reste. Annonce
  « NOUVEAU CONTACT DÉBLOQUÉ : … » puis courrier de présentation, une seule fois.

  | Contact | Niveau | Réputation | Quête terminée |
  |---|---|---|---|
  | ECHO-7 | 1 (dès le départ) | 0 | — |
  | R4Z0R | 2 | 10 | — |
  | Phoenix | 4 | 50 | Réseaux d'Information |
  | AURA | 5 | 50 | L'œil du Cyclone |

  Les cinq autres contacts (Shadow Broker, Neon Angel, Ghost Walker, Data Miner, Nexus Insider) n'ont pas de fiche (phase
  4.2) : ils ont un nom et restent verrouillés (`nh_contact_written`). « Réseaux d'Information » exige elle-même 50 de
  réputation (et en verse 40) : un joueur qui la termine a donc déjà de quoi débloquer Phoenix, puis AURA (`test_quests.c` rejoue
  la campagne).
- **Conversation** (`nh_contact_talk`) : un menu de sujets propre au contact, qui *revient* jusqu'à « Terminer », `0`, une
  ligne vide ou la fin de l'entrée (une saisie invalide est dite, puis le menu est réaffiché). Un sujet est une réplique
  (`nh_speak`), le commentaire du niveau du joueur, la **mission en cours de ce contact** (un objectif de quête actif dont
  `NhQuestDef.contact` est lui ; pendant le tutoriel, ECHO-7 redit aussi la consigne de l'étape, `nh_tutorial_repeat`), l'ouverture de
  la boutique (R4Z0R) ou son dossier. Chaque conversation est comptée (`interactions_count`) et émet `NH_EV_CONTACT_MET`,
  que l'objectif « Contacter R4Z0R » lit.
- **Courriers** : `nh_mail_send` dépose un modèle et l'annonce (« Nouveau message de … »). `messages` liste la boîte
  (non lus marqués NOUVEAU), `read N` ouvre un message (et le marque lu). Le message de bienvenue d'ECHO-7 est dans la boîte dès
  `init_contact_system` ; les trois autres partent au déblocage de leur contact.
- **Numérotation** : `contacts` numérote les contacts *débloqués* (1 = le premier dans l'ordre de la table) ; le numéro tapé à
  l'invite, `contact <n>` ou `contact <nom>` (sans tenir compte de la casse) désignent le même contact, la complétion TAB propose
  les noms.
- **Répliques** (`nh_speak` dans `ui/term.c`) : « NOM » texte », le nom en couleur, le texte coupé à la largeur du terminal avec
  retrait sous le premier mot ; `nh_echo_say` (tutoriel) l'utilise aussi, ECHO-7 parle donc de la même façon partout.
- **Sauvegarde** : voir « Sauvegarde ». Rien de textuel n'est écrit, seulement des états et des numéros de modèle.

## Démarrage, menu, prologue et tutoriel

Ordre de `main()` : valeurs par défaut d'après l'environnement → options de la ligne de commande →
dossier de données et `settings.cfg` → application des réglages → menu (ou `--new`) → interface fixe →
annonce du tutoriel → `game_loop`.

- **Priorité des réglages** : environnement (`$LANG`, `NO_COLOR`) < `settings.cfg` < option explicite
  de la ligne de commande. `NhConfig` retient ce qui a été imposé (`lang_set`, `color_set`,
  `fast_set`, `hud_set`) ; `nh_config_apply_settings()` n'applique le fichier qu'au reste. `NO_COLOR`
  éteint les couleurs même si le fichier les allume (seul `--color` le contredit). Le menu modifie à
  la fois la configuration de la session et les réglages enregistrés, mais ne persiste que le champ
  qu'il vient de changer : un `--no-color` de la session n'est jamais écrit dans le fichier.
- **Menu** (`menu.c`) : lit ses choix par `io.c` (EOF = statut `NH_START_EOF`, jamais une boucle),
  tourne *avant* l'interface fixe. « Continuer » n'est proposé que si `nh_save_peek()` réussit ; une
  sauvegarde illisible est signalée comme telle, et « Nouvelle partie » demande confirmation dès
  qu'un fichier existe, lisible ou non.
- **Prologue** (`intro.c`) : le nom du héros se choisit *dans la fiction* (ECHO-7 le demande, le confirme).
  `nh_clean_name()` retire les caractères de contrôle (dont ESC, C1), les octets UTF-8 invalides et les
  espaces superflus, et coupe à 20 *caractères* ; un nom vide vaut `NH_DEFAULT_NAME` (« Case »). La
  même fonction assainit le nom relu depuis une sauvegarde : un fichier modifié à la main ne peut pas
  injecter de séquence d'échappement dans le terminal.
- **Tutoriel** (`tutorial.c`) : *pas un écran à part* mais la première mission, jouée dans la vraie
  boucle. `GameState.tutorial {step, done}` est une machine à états ; `nh_dispatch()` appelle
  `nh_tutorial_on_command()` après chaque ligne. Les étapes « lance telle commande » (`quests`, `help`,
  `scan`, `status`) ne se valident que par la commande qui vient de réussir ; les étapes d'état (niveau 2,
  `localhost` compromis, une réduction d'alerte **appliquée** — `AlertSystem.reductions_done`, qui ignore
  l'annulation du menu) se lisent dans le jeu, donc s'enchaînent d'elles-mêmes si le joueur a pris de
  l'avance. Rien n'est bloqué : une commande inconnue, verrouillée ou ratée déclenche un rappel
  d'ECHO-7. La récompense (100 ¢, 10 de réputation) n'est versée qu'à la dernière étape, une fois
  (`done`), par le moteur de quêtes (`nh_quest_complete`, voir « Événements et quêtes ») ; le passer la clôt
  sans rien verser. Un `GameState` neuf n'a *pas* de tutoriel actif : c'est le prologue qui le
  lance ou le passe.
- **Ordre d'annonce** : `nh_hud_start()` repousse dans l'historique tout ce qui est déjà affiché ; la
  première consigne (`nh_tutorial_announce`) est donc émise *après* lui, sinon elle disparaîtrait
  au-dessus de l'écran. Les répliques d'ECHO-7 sont coupées à la largeur du terminal avec retrait
  (`nh_wrap_text`) ; sur un tube (`nh_wrap_width() == 0`) le texte reste sur une ligne.

## Sauvegarde

- **Format** (`save.c` + `kv.c`) : texte `clé=valeur`, une paire par ligne, lisible et éditable. La première
  ligne porte la version (`neon-hack-save=1`), la dernière `end=1` prouve que rien n'est tronqué. On
  n'enregistre que ce que le jeu *modifie* (joueur, indicateurs des nœuds, alerte, quêtes, contacts,
  tutoriel…) : les tables fixes sont reconstruites par `init_game`, une mise à jour du contenu ne
  casse donc pas les sauvegardes existantes. Pas de vidage de structure (dépendant du compilateur et
  ouvert aux indices hors tableau).
- **Quêtes et achats** : `quest.N.status` et `quest.N.obj.M` (avancement de chaque objectif, rogné à sa cible au
  chargement ; une quête terminée est toujours relue complète), `shop.bought` (masque des objets achetés, que les
  quêtes lisent). Les compteurs d'origine (`quests.active`, `quests.completed`, `quest.N.done`…) ne sont plus écrits et
  sont ignorés à la lecture : ils se déduisent des statuts.
- **Boutique, contacts, courrier** : `player.key`, `player.xp_boost`, `alert.vpn`, `alert.proxy_left`, `alert.ghost` (effets
  des objets) ; `contact.N.flags` (bit 0 : débloqué) et `contact.N.interactions` ; `mail.count` puis `mail.N.id` (numéro de
  modèle) et `mail.N.read`. Toute valeur présente mais hors bornes, un modèle inconnu ou en double donnent `NH_SAVE_CORRUPT`.
  Les anciennes clés (`alert.proxy`, `contacts.active`, `inbox.read`, bit « découvert » des contacts) sont lues ou ignorées :
  un ancien masque `inbox.read` dont le bit 0 est levé marque lu le courrier de bienvenue.
- **Compatibilité** : une clé absente garde sa valeur par défaut, une clé inconnue est ignorée ;
  `NH_SAVE_VERSION` ne monte que pour un changement incompatible. Une version supérieure à celle du
  jeu donne `NH_SAVE_TOO_NEW`, jamais un chargement approximatif.
- **Chargement transactionnel** : partir d'un état neuf, valider chaque valeur (bornes strictes,
  `nh_kv_int`), et ne remplacer la partie en cours qu'à la toute fin. Une valeur hors bornes donne
  `NH_SAVE_CORRUPT` et laisse `gs` intact.
- **Écriture atomique** (`storage.c`) : fichier temporaire, `fsync`, puis renommage (`MoveFileEx` sous
  Windows) ; le dossier est créé au besoin. Un disque plein ou une coupure ne laisse jamais une
  sauvegarde à moitié écrite.
- **Quand** : après chaque commande (`game_loop`), sauf si la partie est perdue — *Continuer* reprend alors
  la sauvegarde d'avant la commande fatale —, et à `quit` / `save`. Un échec est signalé une fois.
  Sans `save_path` (tests, aucun dossier de données) rien n'est écrit.
- **Emplacement** : `--data-dir` > `%APPDATA%/neon-hack` > `$XDG_DATA_HOME/neon-hack` (absolu seulement,
  comme le veut XDG) > `~/.local/share/neon-hack`. Fichiers : `savegame.sav`, `settings.cfg`.
- **Limites connues** : un seul emplacement ; `--new` remplace la sauvegarde sans confirmation (le menu,
  lui, la demande) ; les branches Windows de `storage.c` n'ont jamais été compilées ; macOS utilise
  `~/.local/share` faute d'un choix plus idiomatique.

## Interface fixe (HUD)

`src/ui/hud.c` réserve la première et la dernière ligne du terminal pour deux barres et confie
le reste à la **région de défilement** ANSI (`ESC[2;N r`) : le terminal fait défiler le texte tout
seul entre les barres, sans bibliothèque (pas de ncurses). Le module ne connaît pas le jeu :
`nh_refresh_hud()` (commands.c) lui passe un `NhHudData` avant chaque prompt.

- **Activation** : vrai terminal (`isatty`), au moins 80×24, `TERM` ≠ `dumb`, pas de `--no-hud`.
  Sinon toutes les fonctions `nh_hud_*` sont sans effet et le jeu se comporte comme avant
  (c'est ce qui laisse les tests e2e sur tube et `--fast` inchangés).
- **Redimensionnement** : la taille est relue avant chaque prompt ; sous 80×24 le HUD se coupe.
  Pendant l'attente d'une saisie, la fenêtre n'est redessinée qu'à la prochaine validation.
- **Sortie propre** : `nh_hud_stop()` (appelé en fin de partie, par `atexit`, et par un gestionnaire
  SIGINT/SIGTERM) rétablit la région de défilement, sinon le terminal resterait figé.
- **`clear`** n'efface que la zone de texte. Au démarrage le contenu déjà affiché est repoussé dans
  l'historique du terminal plutôt qu'effacé.
- **Défilement de l'historique** : les barres appartiennent à l'écran normal du terminal ; remonter avec la molette ou
  Maj+PgUp les fait défiler avec le reste (rien n'est faisable côté jeu). D'où la règle : *un écran affiché d'un seul tenant
  doit tenir dans la zone de texte* (la vitrine de la boutique calcule son budget avec `nh_hud_active()` et
  `nh_term_size()`), pour que le joueur n'ait pas besoin de remonter le lire.
- **Limite connue** : un panneau *à droite* du texte n'est pas possible avec cette technique (la
  région de défilement occupe toute la largeur) ; il demanderait un historique de texte tenu par le
  jeu (option B du plan). Le code Windows (`nh_term_size`) n'a jamais été compilé.

## Saisie de ligne et complétion (TAB)

`src/ui/lineedit.[ch]` remplace la lecture de la ligne de commande par une saisie « comme dans un vrai terminal » :
flèches gauche/droite, Début/Fin, Suppr, Ctrl+A/E/B/F, Ctrl+U/K/W, historique (↑/↓, Ctrl+P/N) et complétion par TAB à la
manière de bash. Deux couches :

- **Le cœur est pur** (`nh_le_*`) : il reçoit des octets (`nh_le_feed`), tient la ligne à jour et *écrit dans un tampon* ce
  qu'il faut afficher. Aucune entrée-sortie : `test_lineedit.c` le joue octet par octet, sans terminal.
- **La colle** (`nh_lineedit_read`) passe le terminal en mode brut pendant *la seule durée d'une lecture* (`ICANON`, `ECHO`
  et `IEXTEN` coupés, `ISIG` gardé : Ctrl+C et Ctrl+Z gardent leur effet), lit avec `read`, écrit avec `write`, et **rétablit
  toujours le terminal** (fin normale, Ctrl+D, et gestionnaires SIGINT/SIGTERM/SIGHUP qui restaurent puis passent la main au
  gestionnaire précédent, dont celui du HUD). Un ESC isolé est distingué d'une séquence de touche par un délai de 40 ms.
- **Repli** : si l'entrée ou la sortie n'est pas un terminal, si `TERM=dumb`, si l'entrée est un flux de test
  (`nh_io_uses_stdin()` faux) ou sous Windows, `nh_lineedit_read` appelle `nh_read_line` : rien ne change pour les tubes, les
  scripts et les tests e2e (un TAB y est un caractère comme un autre). C'est la seule exception à la règle « pas de lecture
  directe de `stdin` » ci-dessous.
- **Affichage sans jamais remonter le curseur** : la ligne tient toujours sur *une* rangée ; trop longue, la fenêtre visible
  défile pour garder le curseur en vue. Un redessin est `\r`, avance au-delà du prompt, la partie visible, `ESC[K`, puis
  repositionnement ; il ne réécrit pas le prompt, n'écrit jamais dans la dernière colonne (pas de retour automatique) et
  compte en largeur d'affichage (UTF-8, caractères larges). Pas d'« effacer jusqu'en bas » : il emporterait la barre du bas
  du HUD. Le prompt est d'abord ramené en début de rangée (astuce `PROMPT_SP` de zsh : `cols-1` espaces puis `\r`), car des
  messages comme `[ALERTE +1]` laissent le curseur en milieu de ligne.
- **TAB** : un seul candidat → il est complété (avec une espace après un nom de commande) ; plusieurs → on avance jusqu'au plus
  long préfixe commun ; sans progression possible, une sonnerie, puis au TAB suivant la liste des candidats en colonnes
  (comme `ls`), le prompt étant ré-affiché en dessous. La casse tapée est ignorée, le mot inséré prend celle du candidat.
- **Les candidats viennent du jeu** (`complete.c`, `nh_complete_line`), pas du module d'édition : au premier mot, les commandes
  disponibles *maintenant* — mêmes règles que `help` et que la barre du bas (niveau, déblocage, pas les commandes cachées) ;
  un alias n'est proposé que si aucun nom officiel ne convient. À l'argument, selon `NhCommand.arg` : systèmes découverts par
  `scan` (`bruteforce`, `traceroute`…), contacts débloqués (`contact`), numéros des messages reçus (`read`). L'argument est
  *tout* le reste de la ligne (un contact peut avoir une espace dans son nom). Le joueur ne se voit jamais proposer ce qu'il
  ne peut pas utiliser, ni découvre par TAB ce qu'il n'a pas encore trouvé.
- **Historique** : `NH_LE_HISTORY` lignes (32), propre à la session, jamais sauvegardé ; les lignes vides et les doublons
  consécutifs sont ignorés ; la ligne en cours de frappe est mise de côté pendant qu'on le parcourt.
- **Ajouter une commande avec argument** : renseigner `arg` dans sa ligne de `k_commands` ; un nouveau genre d'argument =
  une valeur de `NhArgKind` et un `case` dans `complete_argument()` (le compilateur signale l'oubli).
- Les questions du menu, du prologue et de la boutique restent lues par `nh_read_line` : seule l'invite de la boucle de jeu
  est éditable.

## Règles du socle

- **Aucune lecture directe de `stdin`** dans le nouveau code : tout passe par `io.c` (seule exception : le mode brut de
  `lineedit.c`, qui n'est utilisé que sur un vrai terminal et retombe sur `io.c` sinon).
  Une entrée fermée (EOF) est un statut à traiter, jamais une boucle infinie.
- **Aucune pause directe** (`sleep`, `usleep`) : `nh_sleep_ms()` respecte `--fast` et
  se désactive seul quand la sortie n'est pas un terminal.
- **Pas de `rand()`** dans le nouveau code : `NhRng`, avec une graine explicite
  (`--seed`), pour que les tests soient déterministes et identiques d'un système à l'autre.
- **Textes dans `strings.def`**, une ligne par texte avec les deux langues côte à côte
  (`NH_STR(CLÉ, "français", "english")`). Une traduction manquante ne compile pas, et
  `tests/unit/test_i18n.c` vérifie que les deux versions ont les mêmes marqueurs
  `printf` (`%s`, `%d`…).
- **Largeur d'affichage ≠ nombre d'octets** : pour aligner du texte accentué, des emoji
  ou des boîtes, utiliser `nh_display_width()` / `nh_pad()`, jamais `%-20s`.

## Commandes

```bash
make              # compile ./neon_hack
make test         # tests unitaires + bout en bout
make asan         # idem avec AddressSanitizer + UBSan
```

## Écarts assumés par rapport au plan initial

- **Un seul fichier de textes à deux colonnes** plutôt que `lang_fr.c` / `lang_en.c` :
  même garantie de complétude, moins de duplication.
- **Makefile conservé** (au lieu de CMake) tant que le projet n'a qu'un exécutable ;
  à réévaluer pour la matrice Windows/macOS de la phase 8.
- Le mode `--no-color` est appliqué à la couche commandes (aide, statut, prompt) mais pas
  encore aux écrans d'origine (logo, boutique, contacts…) : leurs couleurs sont des
  constantes de compilation (`legacy_colors.h`). Il le sera au fur et à mesure du portage.
