# Architecture

Neon Hack est en cours de refonte selon une approche **« strangler »** : le nouveau
socle se construit à côté du code d'origine, le jeu reste jouable à chaque étape,
et les modules d'origine sont portés un par un avant d'être supprimés.

## Organisation actuelle

```
src/main.c         point d'entrée : options, langue, création du GameState, boucle
src/game/          logique de jeu (GameState unique, plus aucune variable globale d'état)
  game.[ch]          GameState, init_game, boucle de jeu, progression (gain_experience)
  progression.[ch]   courbe d'expérience, niveaux 1-6, déblocages, récompenses uniques ; nh_grant_xp() est le SEUL point d'entrée
  world.[ch]         le monde : graphe unique de systèmes (relais, découverte), état persistant, chances de succès, récompense unique
  alert.[ch]         alerte unique 0-100 : hausse, refroidissement, seuils, méthodes de réduction, affichage
  commands.[ch]      table de commandes, dispatch, aide, commandes système (help/status/quit/clear)
  cmd_hacking.c      commandes de hacking classiques (scan, bruteforce, decrypt, backdoor…)
  cmd_world.c        boutique, quêtes, contacts, messages, laylow
  cmd_advanced.c     hacking avancé (advhack, aiassist, neuralsync, temporalhack…)
  shop, alert_system, quest_system, contacts, advanced_hacking   modules d'origine (à porter, phase 3)
src/core/          socle neuf, testé, sans état de jeu
  platform.[ch]      pauses, mode rapide, console (Windows : UTF-8 + ANSI), détection du terminal
  io.[ch]            lecture de lignes et d'entiers sûre, EOF géré
  parse.[ch]         découpe « commande argument », comparaison sans casse
  utf8.[ch]          coupe propre d'une chaîne UTF-8 tronquée
  rng.[ch]           générateur PCG32 reproductible (graine, tirages sans biais)
  config.[ch]        options de ligne de commande, variables d'environnement
src/ui/term.[ch]   couleurs ANSI, largeur d'affichage UTF-8, remplissage de colonnes, jauge, troncature
src/ui/hud.[ch]    interface fixe : barres haut/bas + zone de texte défilante (région de défilement ANSI)
src/i18n/          textes français/anglais (strings.def, i18n.[ch])
tests/unit/        tests unitaires (un exécutable par fichier ; test_commands.c teste la couche jeu)
tests/e2e/run.sh   tests de bout en bout du jeu compilé (sorties redirigées)
tests/e2e/pty_hud.py   tests de l'interface fixe dans un vrai pseudo-terminal + mini-émulateur d'écran
```

Le code neuf (`src/core`, `src/ui`, `src/i18n`, `src/main.c`, `src/game/commands.c`, `src/game/alert.c`, `src/game/progression.c`, `src/game/world.c`)
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
  `strings.def` (sans elle, le projet ne compile pas).
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
- **Limite connue** : un panneau *à droite* du texte n'est pas possible avec cette technique (la
  région de défilement occupe toute la largeur) ; il demanderait un historique de texte tenu par le
  jeu (option B du plan). Le code Windows (`nh_term_size`) n'a jamais été compilé.

## Règles du socle

- **Aucune lecture directe de `stdin`** dans le nouveau code : tout passe par `io.c`.
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
