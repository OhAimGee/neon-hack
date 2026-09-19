# 🌆 Neon Hack

RPG textuel cyberpunk pour le terminal, écrit en C. Vous êtes un hacker novice à Neo-Tokyo, en 2087, face à la mégacorporation Nexus Corp et à son mystérieux Projet Aurora.

> Jeu généré par IA en 2025 et laissé inachevé. Il est **en cours de refonte** pour devenir un vrai jeu publiable (voir [Statut](#statut)).

*English: a cyberpunk terminal RPG written in C, AI-generated in 2025 and currently being rebuilt into a releasable game (French only for now, English translation planned). Build with `make`, run with `./neon_hack`.*

## Statut

Le jeu **se compile et se lance**, mais plusieurs systèmes sont affichés sans être encore branchés sur le gameplay. Le détail, vérifié en jouant :

| Fonctionne | Ne fonctionne pas encore |
|---|---|
| Boucle de jeu, `scan`, `bruteforce`, `decrypt` | Niveau d'alerte réel (il reste à 0) |
| Niveaux et déblocage de commandes | Effets des achats en boutique |
| Ambiance, ASCII art, lore | Progression des quêtes ; 3 contacts sur 4 |
| Boutique, contacts, quêtes : affichage | `exploit`, `stealthmode`, sauvegarde |

**Bug connu :** fermer l'entrée (Ctrl+D) fait tourner le jeu en boucle. Quittez avec `quit`.

La refonte (architecture unifiée, tests, sauvegarde, français/anglais, releases binaires) se déroule sur la branche `refonte/v1`. La version d'origine reste consultable via le tag `legacy-v2.087`.

## Compiler et lancer

Prérequis : `gcc` (ou `clang`), `make`, un terminal UTF-8 avec couleurs ANSI (Linux, macOS, ou Windows via WSL).

```bash
make        # compile
./neon_hack # lance le jeu
make run    # compile puis lance
```

## Jouer

Tapez `help` en jeu : l'aide n'affiche que les commandes déjà débloquées.

- **Départ** : `scan` (débloqué), puis répétez-le pour gagner de l'expérience ; 5 scans mènent au niveau 2 et débloquent `bruteforce`.
- **Hacking** : `scan`, `bruteforce <cible>`, `decrypt <texte>`, `backdoor`, `traceroute`, `uploadvirus`.
- **Hacking avancé** : `advhack <cible>`, `analyzedefenses`, `socialeng`, `aiassist`, `neuralsync`, `quantumdecrypt`, `temporalhack`.
- **Monde** : `shop`, `laylow`, `quests`, `contacts`, `contact <n° ou nom>`, `messages`, `read <n°>`.
- **Système** : `status`, `clear`, `quit`.

Un message à décrypter pour essayer : `decrypt WKLV#LV#D#WHVW` (chiffre de César, décalage −3).

## Structure

```
neon_hack.c        boucle de jeu et commandes (code d'origine)
src/game/          modules d'origine : boutique, alerte, quêtes, contacts, hacking avancé
tests/, *.sh       scripts d'origine (sans assertions, à remplacer)
docs/legacy/       rapports générés à l'époque (peu fiables, gardés pour mémoire)
```

## Licence

[CC0 1.0 Universal](LICENSE) : domaine public, vous pouvez copier, modifier et redistribuer le projet sans condition.
