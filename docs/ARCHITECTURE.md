# Architecture

Neon Hack est en cours de refonte selon une approche **« strangler »** : le nouveau
socle se construit à côté du code d'origine, le jeu reste jouable à chaque étape,
et les modules d'origine sont portés un par un avant d'être supprimés.

## Organisation actuelle

```
src/main.c         point d'entrée : options, langue, création du GameState, boucle
src/game/          logique de jeu (GameState unique, plus aucune variable globale d'état)
  game.[ch]          GameState, init_game, boucle de jeu, progression (gain_experience)
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
src/ui/term.[ch]   couleurs ANSI, largeur d'affichage UTF-8, remplissage de colonnes
src/i18n/          textes français/anglais (strings.def, i18n.[ch])
tests/unit/        tests unitaires (un exécutable par fichier ; test_commands.c teste la couche jeu)
tests/e2e/run.sh   tests de bout en bout du jeu compilé
```

Le code neuf (`src/core`, `src/ui`, `src/i18n`, `src/main.c`, `src/game/commands.c`, `src/game/alert.c`)
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
- `exploit` n'est volontairement pas dans la table : elle n'a pas de handler
  (phase 3). Une commande non implémentée n'est pas listée plutôt que d'être fantôme.

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
