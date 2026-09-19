# 🌆 Neon Hack

RPG textuel cyberpunk pour le terminal, écrit en C. Vous êtes un hacker novice à Neo-Tokyo, en 2087, face à la mégacorporation Nexus Corp et à son mystérieux Projet Aurora.

> Jeu généré par IA en 2025 et laissé inachevé. Il est **en cours de refonte** pour devenir un vrai jeu publiable (voir [Statut](#statut)).

**Français** · [English](README.en.md)

Le jeu se joue en français et en anglais (`--lang en`, ou l'entrée *Langue* du menu). Quelques écrans ne sont pas encore traduits : voir le [Statut](#statut).

## Statut

Le jeu **se compile et se lance**, mais plusieurs systèmes sont affichés sans être encore branchés sur le gameplay. Le détail, vérifié en jouant :

| Fonctionne | Ne fonctionne pas encore |
|---|---|
| Boucle de jeu, `scan`, `bruteforce`, `decrypt` | Effets des achats en boutique (seul le Street Cred Booster en a un) |
| Niveaux et déblocage de commandes | 3 contacts sur 4 |
| **Quêtes** : les quatre premières se jouent de bout en bout (tutoriel, *Baptême du Feu*, *Réseaux d'Information*, *L'œil du Cyclone*), journal `quests` FR/EN, objectifs mesurés sur l'état du jeu, récompenses versées une seule fois, annonces de chapitre | Quêtes 5 à 9 et épilogue (pas encore écrits) |
| **Alerte 0-100** (jauge, refroidissement, boutique fermée, game over, `laylow`) ; **niveaux 1-6** avec courbe d'expérience et déblocages ; **monde unifié** (7 systèmes reliés, `exploit`, récompenses uniques) | `stealthmode` |
| **Menu de lancement** (continuer, nouvelle partie, langue, options) ; **sauvegarde automatique** ; **prologue et tutoriel** menés par ECHO-7, où l'on choisit le nom du héros (« Case » par défaut) | Un seul emplacement de sauvegarde ; l'écran d'accueil est encore en français seulement |
| **Saisie « comme dans un vrai terminal »** : complétion par TAB (commandes disponibles, systèmes découverts, contacts, messages), édition de la ligne, historique (↑/↓) | Complétion des objets de la boutique, recherche dans l'historique (Ctrl+R), historique conservé d'une session à l'autre ; testée sous Linux seulement, et sur Windows natif la saisie retombe sur une lecture simple |
| **Boutique de R4Z0R** : tout le catalogue tient à l'écran, sur deux colonnes si le terminal est assez large | Messages d'achat encore en français seulement |
| Fin d'entrée (Ctrl+D) et saisies invalides gérées ; `--seed`, `--fast`, `--lang`, tests automatisés | Équilibrage général (valeurs provisoires) ; farm de crédits par la boutique/`advhack` |
| Ambiance, ASCII art, lore ; affichage des contacts | Texte anglais complet (aide, statut, alerte, monde, quêtes et vitrine de la boutique sont traduits, le reste non) |

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
| `--lang fr\|en` | langue (défaut : le dernier choix du menu, sinon d'après `$LANG`) ; certains écrans ne sont pas encore traduits |
| `--new` | nouvelle partie tout de suite, sans menu (l'ancienne sauvegarde est remplacée à la fin du prologue) |
| `--data-dir DIR` | dossier des sauvegardes et réglages (défaut : `$XDG_DATA_HOME/neon-hack`, sinon `~/.local/share/neon-hack` ; `%APPDATA%\neon-hack` sous Windows) |
| `--no-hud` | désactive les barres fixes (elles n'apparaissent de toute façon que sur un vrai terminal d'au moins 80×24) |
| `--no-color` | désactive les couleurs (aide, statut, prompt et boutique ; pas encore les autres écrans d'origine) |
| `--version`, `--help` | version, aide |

Une option donnée sur la ligne de commande (`--lang`, `--no-color`, `--color`, `--fast`, `--no-hud`) l'emporte, pour la session, sur les réglages enregistrés par le menu ; ceux-ci l'emportent à leur tour sur l'environnement (`$LANG`, `NO_COLOR`).

## Développement

```bash
make test   # tests unitaires + tests de bout en bout du jeu ; si python3 est présent, aussi ceux de l'interface,
            # de la boutique et de la saisie, joués dans un vrai pseudo-terminal
make asan   # les mêmes, compilés avec AddressSanitizer + UBSan
```

Voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour l'organisation du code et les règles de la refonte, et [docs/ROADMAP.md](docs/ROADMAP.md) pour les phases à venir.

## Jouer

Sur un terminal d'au moins 80×24, une **barre d'état** reste fixée en haut (nom, niveau, crédits, jauge d'alerte) et une **barre de commandes** en bas ; le texte défile entre les deux. Elle disparaît d'elle-même si la fenêtre devient trop petite, si la sortie est redirigée ou avec `--no-hud`.

**La saisie se comporte comme celle d'un vrai terminal.** La touche **TAB complète** ce que vous tapez : au premier mot, les commandes que vous pouvez utiliser *maintenant* (les mêmes que dans `help`, jamais une commande verrouillée) ; après le nom d'une commande, son argument : systèmes découverts par `scan` (`bruteforce`, `traceroute`…), contacts débloqués (`contact`), numéros des messages reçus (`read`). Un seul choix possible : il est inséré. Plusieurs : on avance jusqu'à leur partie commune, et un second TAB les liste, comme dans bash. La casse est ignorée (`SCA` + TAB donne `scan `). On peut aussi éditer la ligne : `←` `→`, `Début`, `Fin`, `Suppr`, `Ctrl+A` / `Ctrl+E` (début / fin), `Ctrl+U` / `Ctrl+K` (efface avant / après le curseur), `Ctrl+W` (efface le mot) ; `↑` `↓` (ou `Ctrl+P` / `Ctrl+N`) rappellent les 32 dernières lignes. Cela ne concerne que l'invite de commandes, et seulement sur un vrai terminal : avec une entrée redirigée (tube, fichier, tests), les lignes sont lues telles quelles.

**Au lancement**, le menu propose : *Continuer* (avec le nom, le niveau et les crédits de la partie sauvegardée), *Nouvelle partie*, *Langue* (Français / English, appliquée sur-le-champ), *Options* (couleurs, animations du texte, barres fixes) et *Quitter*. Langue et options sont retenues d'une session à l'autre ; Entrée seule choisit *Continuer* s'il y a une sauvegarde, *Nouvelle partie* sinon.

**Une nouvelle partie** commence par un court prologue : votre contact, **ECHO-7**, vous demande votre *handle*. C'est ainsi que se choisit le nom du héros (Entrée pour « Case », 20 caractères au plus, confirmation demandée). ECHO-7 propose ensuite un **tutoriel** : une mission pas à pas dans la vraie partie (`quests`, `help`, `scan`, `status`, atteindre le niveau 2, `bruteforce localhost`, `laylow`). Il commente chaque action, rappelle l'objectif si vous vous égarez (commande inconnue, verrouillée ou ratée) et ne bloque rien ; `quests` affiche l'avancement de la mission. La terminer rapporte 100 ¢ et 10 de réputation, une seule fois. On peut aussi répondre « Je me débrouille » et s'en passer (le tutoriel est alors clos sans récompense).

**Les quêtes** démarrent d'elles-mêmes dès que leur niveau et leurs prérequis sont atteints, et le disent à l'écran (« NOUVELLE QUÊTE », avec une annonce de chapitre pour la première d'un chapitre). Elles se mesurent sur l'état du jeu — systèmes compromis, fichiers extraits, achats, contacts rencontrés, réputation, jalons — et non sur des compteurs : ce que vous avez déjà fait compte. `quests` ouvre le journal ; donner le numéro d'une quête en affiche le détail (contexte, objectifs, récompenses). Certains objectifs sont des *conditions* qui se vérifient au moment de conclure (« maintenir l'alerte sous 50 » : `laylow` la fait baisser) ; d'autres sont secrets jusqu'à leur accomplissement. Une quête terminée verse ses récompenses (crédits, réputation, expérience) **une seule fois** et peut en ouvrir une autre. Quatre quêtes sont jouables pour l'instant ; les suivantes viendront avec la phase 4 de la [feuille de route](docs/ROADMAP.md).

**La boutique de R4Z0R** (`shop`) affiche tout son catalogue d'un coup, sans qu'il faille remonter dans l'historique du terminal : dix objets, chacun avec son numéro, son prix, un résumé et son état (disponible, niveau requis, il manque des crédits, épuisé). Dès 72 colonnes de terminal, ils se répartissent sur **deux colonnes** ; sur un terminal plus étroit ou plus bas, la présentation se resserre (cartes serrées, puis simple liste) au lieu de déborder. Tapez le numéro de l'objet pour l'acheter, `0` pour partir. Pour l'instant seul le *Street Cred Booster* a un effet : les neuf autres arrivent avec le lot 3.4 de la feuille de route.

**Sauvegarde** : la partie est enregistrée après chaque commande (et par `quit` ou `save`) dans `savegame.sav`, un fichier texte lisible, écrit de façon atomique : une coupure en pleine écriture laisse l'ancienne sauvegarde intacte. Un fichier corrompu, tronqué ou écrit par une version plus récente est refusé sans rien charger à moitié. Une partie perdue (alerte à 100) n'écrase pas la dernière sauvegarde : *Continuer* reprend juste avant la commande fatale.

Tapez `help` en jeu : l'aide n'affiche que les commandes déjà débloquées (casse et espaces ignorés ; `upload_virus`, `ai_hack`, `quantum_decrypt`, `exit` fonctionnent aussi). Dans le doute, TAB propose ce qui est possible.

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
src/game/          état de jeu unique, table de commandes, bus d'événements, quêtes, vitrine de la boutique, complétion ; modules d'origine (boutique, contacts, hacking avancé) en cours de portage
src/core/, ui/, i18n/   socle testé (entrées/sorties, options, RNG, terminal, interface fixe, saisie de ligne, textes FR/EN)
tests/             tests unitaires (unit/) et de bout en bout (e2e/)
docs/              architecture et feuille de route ; docs/legacy/ = rapports générés à l'époque (peu fiables)
```

## Licence

[CC0 1.0 Universal](LICENSE) : domaine public, vous pouvez copier, modifier et redistribuer le projet sans condition.
