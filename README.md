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
| **Alerte 0-100** (jauge, refroidissement, boutique fermée, game over, `laylow`) ; **niveaux 1-6** avec courbe d'expérience et déblocages ; **monde unifié** (7 systèmes reliés, `exploit`, récompenses uniques) | `stealthmode` |
| **Menu de lancement** (continuer, nouvelle partie, langue, options) ; **sauvegarde automatique** ; **prologue et tutoriel** menés par ECHO-7, où l'on choisit le nom du héros (« Case » par défaut) | Un seul emplacement de sauvegarde ; le journal de quêtes d'origine et l'écran d'accueil sont encore en français seulement |
| Fin d'entrée (Ctrl+D) et saisies invalides gérées ; `--seed`, `--fast`, `--lang`, tests automatisés | Équilibrage général (valeurs provisoires) ; farm de crédits par la boutique/`advhack` |
| Ambiance, ASCII art, lore ; affichage boutique/contacts/quêtes | Texte anglais complet (aide, statut, alerte et monde sont traduits, le reste non) |

La refonte (architecture unifiée, tests, sauvegarde, français/anglais, releases binaires) se déroule sur la branche `refonte/v1`. La version d'origine reste consultable via le tag `legacy-v2.087`. La suite prévue, phase par phase, jusqu'à la v1.0 (campagne complète, multiplateforme) est dans la [feuille de route](docs/ROADMAP.md).

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
| `--lang fr\|en` | langue (défaut : le dernier choix du menu, sinon d'après `$LANG`) ; seuls quelques messages sont traduits pour l'instant |
| `--new` | nouvelle partie tout de suite, sans menu (l'ancienne sauvegarde est remplacée à la fin du prologue) |
| `--data-dir DIR` | dossier des sauvegardes et réglages (défaut : `$XDG_DATA_HOME/neon-hack`, sinon `~/.local/share/neon-hack` ; `%APPDATA%\neon-hack` sous Windows) |
| `--no-hud` | désactive les barres fixes (elles n'apparaissent de toute façon que sur un vrai terminal d'au moins 80×24) |
| `--no-color` | désactive les couleurs (aide, statut et prompt ; pas encore les écrans d'origine) |
| `--version`, `--help` | version, aide |

Une option donnée sur la ligne de commande (`--lang`, `--no-color`, `--color`, `--fast`, `--no-hud`) l'emporte, pour la session, sur les réglages enregistrés par le menu ; ceux-ci l'emportent à leur tour sur l'environnement (`$LANG`, `NO_COLOR`).

## Développement

```bash
make test   # tests unitaires du socle + tests de bout en bout du jeu
make asan   # les mêmes, compilés avec AddressSanitizer + UBSan
```

Voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour l'organisation du code et les règles de la refonte, et [docs/ROADMAP.md](docs/ROADMAP.md) pour les phases à venir.

## Jouer

Sur un terminal d'au moins 80×24, une **barre d'état** reste fixée en haut (nom, niveau, crédits, jauge d'alerte) et une **barre de commandes** en bas ; le texte défile entre les deux. Elle disparaît d'elle-même si la fenêtre devient trop petite, si la sortie est redirigée ou avec `--no-hud`.

**Au lancement**, le menu propose : *Continuer* (avec le nom, le niveau et les crédits de la partie sauvegardée), *Nouvelle partie*, *Langue* (Français / English, appliquée sur-le-champ), *Options* (couleurs, animations du texte, barres fixes) et *Quitter*. Langue et options sont retenues d'une session à l'autre ; Entrée seule choisit *Continuer* s'il y a une sauvegarde, *Nouvelle partie* sinon.

**Une nouvelle partie** commence par un court prologue : votre contact, **ECHO-7**, vous demande votre *handle*. C'est ainsi que se choisit le nom du héros (Entrée pour « Case », 20 caractères au plus, confirmation demandée). ECHO-7 propose ensuite un **tutoriel** : une mission pas à pas dans la vraie partie (`quests`, `help`, `scan`, `status`, atteindre le niveau 2, `bruteforce localhost`, `laylow`). Il commente chaque action, rappelle l'objectif si vous vous égarez (commande inconnue, verrouillée ou ratée) et ne bloque rien ; `quests` affiche l'avancement de la mission. La terminer rapporte 100 ¢ et 10 de réputation, une seule fois. On peut aussi répondre « Je me débrouille » et s'en passer.

**Sauvegarde** : la partie est enregistrée après chaque commande (et par `quit` ou `save`) dans `savegame.sav`, un fichier texte lisible, écrit de façon atomique : une coupure en pleine écriture laisse l'ancienne sauvegarde intacte. Un fichier corrompu, tronqué ou écrit par une version plus récente est refusé sans rien charger à moitié. Une partie perdue (alerte à 100) n'écrase pas la dernière sauvegarde : *Continuer* reprend juste avant la commande fatale.

Tapez `help` en jeu : l'aide n'affiche que les commandes déjà débloquées (casse et espaces ignorés ; `upload_virus`, `ai_hack`, `quantum_decrypt`, `exit` fonctionnent aussi).

- **Départ** : `scan` (débloqué). Les 5 premiers scans rapportent 5, 4, 3, 2, 1 points : ils mènent au niveau 2 et débloquent `bruteforce` ; les suivants ne rapportent plus rien. L'expérience vient ensuite des hacks (chaque cible ne paie qu'une fois).
- **Niveaux** (expérience cumulée) : 1 Novice · 2 Apprenti (15) · 3 Hacker (60) · 4 Expert (140) · 5 Maître (260) · 6 Légende (420, niveau maximal, débloque `temporalhack`). `status` affiche la progression ; chaque montée annonce les commandes débloquées.
- **Hacking** : `scan`, `bruteforce <cible>`, `decrypt <texte>`, `backdoor`, `traceroute`, `exploit`, `uploadvirus`, `aihack`.
- **Hacking avancé** : `advhack <cible>`, `analyzedefenses`, `socialeng`, `aiassist`, `neuralsync`, `quantumdecrypt`, `temporalhack`.
- **Le réseau** : 7 systèmes, révélés par `scan` selon votre niveau et reliés entre eux. Pour attaquer un système il faut avoir **compromis son relais** (`scan` marque `[ROUTE FERMÉE]`) ; `traceroute` nomme le relais et, une fois un système tracé, `exploit` (niveau 4) perce directement. Chaque système ne paie qu'**une fois** (crédits, expérience, fichiers) : `socialeng` y apporte des accès internes (+15 % sur toutes les attaques). Les méthodes de `advhack` exigent un outil, actif dès que vous en possédez l'équipement (ordinateur quantique, IA assistante, virus, `neuralsync`…).
- **Monde** : `shop`, `laylow`, `quests`, `contacts`, `contact <n° ou nom>`, `messages`, `read <n°>`.
- **Système** : `status`, `save`, `clear`, `quit`.

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
