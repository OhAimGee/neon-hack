# Architecture

Neon Hack est en cours de refonte selon une approche **« strangler »** : le nouveau
socle se construit à côté du code d'origine, le jeu reste jouable à chaque étape,
et les modules d'origine sont portés un par un avant d'être supprimés.

## Organisation actuelle

```
neon_hack.c        code d'origine : boucle de jeu et commandes   (à réécrire, phases 2-3)
src/game/          modules d'origine : boutique, alerte, quêtes, contacts, hacking avancé
src/core/          socle neuf, testé, sans état de jeu
  platform.[ch]      pauses, mode rapide, console (Windows : UTF-8 + ANSI), détection du terminal
  io.[ch]            lecture de lignes et d'entiers sûre, EOF géré
  rng.[ch]           générateur PCG32 reproductible (graine, tirages sans biais)
  config.[ch]        options de ligne de commande, variables d'environnement
src/ui/term.[ch]   couleurs ANSI, largeur d'affichage UTF-8, remplissage de colonnes
src/i18n/          textes français/anglais (strings.def, i18n.[ch])
tests/unit/        tests unitaires du socle (un exécutable par fichier)
tests/e2e/run.sh   tests de bout en bout du jeu compilé
```

Le code neuf est compilé avec `-Wpedantic -Wshadow -Wconversion -Werror`. Le code
d'origine ne l'est pas (il porte encore ses avertissements) : il sera remplacé,
pas corrigé.

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
- Le mode `--no-color` n'est pas encore appliqué aux écrans d'origine : leurs couleurs
  sont des constantes de compilation. Il le sera au fur et à mesure du portage.
