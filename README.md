# 🌆 Neon Hack

RPG textuel cyberpunk pour le terminal, écrit en C. Vous êtes un hacker novice à Neo-Tokyo, en 2087, face à la mégacorporation Nexus Corp et à son mystérieux Projet Aurora.

> Jeu généré par IA en 2025 et laissé inachevé. Il est **en cours de refonte** pour devenir un vrai jeu publiable (voir [Statut](#statut)).

*English: a cyberpunk terminal RPG written in C, AI-generated in 2025 and currently being rebuilt into a releasable game (French only for now, English translation planned). Build with `make`, run with `./neon_hack`.*

## Statut

Le jeu **se compile et se lance**, mais plusieurs systèmes sont affichés sans être encore branchés sur le gameplay. Le détail, vérifié en jouant :

| Fonctionne | Ne fonctionne pas encore |
|---|---|
| Boucle de jeu, `scan`, `bruteforce`, `decrypt` | Effets des achats en boutique |
| Niveaux et déblocage de commandes | Progression des quêtes ; 3 contacts sur 4 |
| **Alerte 0-100** (jauge, refroidissement, boutique fermée, game over, `laylow`) ; **niveaux 1-6** avec courbe d'expérience et déblocages | `exploit`, `stealthmode`, sauvegarde |
| Fin d'entrée (Ctrl+D) et saisies invalides gérées ; `--seed`, `--fast`, `--lang`, tests automatisés | Équilibrage général (valeurs provisoires) ; farm de crédits par la boutique/`advhack` |
| Ambiance, ASCII art, lore ; affichage boutique/contacts/quêtes | Texte anglais complet (aide, statut et alerte sont traduits, le reste non) |

La refonte (architecture unifiée, tests, sauvegarde, français/anglais, releases binaires) se déroule sur la branche `refonte/v1`. La version d'origine reste consultable via le tag `legacy-v2.087`.

## Compiler et lancer

Prérequis : `gcc` (ou `clang`), `make`, un terminal UTF-8 avec couleurs ANSI (Linux, macOS, ou Windows via WSL).

```bash
make        # compile
./neon_hack # lance le jeu
make run    # compile puis lance
```

Options (`./neon_hack --help`) :

| Option | Effet |
|---|---|
| `--seed N` | partie reproductible (graine aléatoire fixée) |
| `--fast` | supprime les pauses d'animation (automatique si la sortie est redirigée) |
| `--lang fr\|en` | langue (défaut : d'après `$LANG`) ; seuls quelques messages sont traduits pour l'instant |
| `--no-hud` | désactive les barres fixes (elles n'apparaissent de toute façon que sur un vrai terminal d'au moins 80×24) |
| `--no-color` | désactive les couleurs (aide, statut et prompt ; pas encore les écrans d'origine) |
| `--version`, `--help` | version, aide |

## Développement

```bash
make test   # tests unitaires du socle + tests de bout en bout du jeu
make asan   # les mêmes, compilés avec AddressSanitizer + UBSan
```

Voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour l'organisation du code et les règles de la refonte.

## Jouer

Sur un terminal d'au moins 80×24, une **barre d'état** reste fixée en haut (nom, niveau, crédits, jauge d'alerte) et une **barre de commandes** en bas ; le texte défile entre les deux. Elle disparaît d'elle-même si la fenêtre devient trop petite, si la sortie est redirigée ou avec `--no-hud`.

Tapez `help` en jeu : l'aide n'affiche que les commandes déjà débloquées (casse et espaces ignorés ; `upload_virus`, `ai_hack`, `quantum_decrypt`, `exit` fonctionnent aussi).

- **Départ** : `scan` (débloqué). Les 5 premiers scans rapportent 5, 4, 3, 2, 1 points : ils mènent au niveau 2 et débloquent `bruteforce` ; les suivants ne rapportent plus rien. L'expérience vient ensuite des hacks (chaque cible ne paie qu'une fois).
- **Niveaux** (expérience cumulée) : 1 Novice · 2 Apprenti (15) · 3 Hacker (60) · 4 Expert (140) · 5 Maître (260) · 6 Légende (420, niveau maximal, débloque `temporalhack`). `status` affiche la progression ; chaque montée annonce les commandes débloquées.
- **Hacking** : `scan`, `bruteforce <cible>`, `decrypt <texte>`, `backdoor`, `traceroute`, `uploadvirus`.
- **Hacking avancé** : `advhack <cible>`, `analyzedefenses`, `socialeng`, `aiassist`, `neuralsync`, `quantumdecrypt`, `temporalhack`.
- **Monde** : `shop`, `laylow`, `quests`, `contacts`, `contact <n° ou nom>`, `messages`, `read <n°>`.
- **Système** : `status`, `clear`, `quit`.

Un message à décrypter pour essayer : `decrypt WKLV#LV#D#WHVW` (chiffre de César, décalage −3).

## Structure

```
src/main.c         point d'entrée
src/game/          état de jeu unique, table de commandes, commandes ; modules d'origine (boutique, alerte, quêtes, contacts) en cours de portage
src/core/, ui/, i18n/   socle testé (entrées/sorties, options, RNG, terminal, textes FR/EN)
tests/             tests unitaires (unit/) et de bout en bout (e2e/)
docs/              architecture ; docs/legacy/ = rapports générés à l'époque (peu fiables)
```

## Licence

[CC0 1.0 Universal](LICENSE) : domaine public, vous pouvez copier, modifier et redistribuer le projet sans condition.
