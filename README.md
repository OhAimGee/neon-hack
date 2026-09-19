# 🌆 Neon Hack - RPG Cyberpunk Terminal

Un jeu de rôle textuel cyberpunk développé en C, où vous incarnez un hacker dans une mégalopole futuriste contrôlée par des corporations.

## 🎯 Synopsis

Dans Neo-Tokyo 2087, vous êtes un hacker novice cherchant à révéler les secrets cachés de Nexus Corp. Infiltrez des réseaux, piratez des systèmes et affrontez des choix critiques qui façonneront votre destin dans cette aventure cyberpunk immersive.

## ⚡ Nouvelles Fonctionnalités v2.087

### 🔥 Système de Hacking Avancé

- **8 méthodes de hacking spécialisées** : Quantum, IA, Neural, Temporal, etc.
- **7 nouvelles commandes** : `advhack`, `stealthmode`, `socialeng`, `aiassist`, etc.
- **Système de progression** par niveaux avec déblocage de capacités
- **Mode furtif** pour réduire la détection
- **Interface neurale** pour améliorer les performances

## Installation et Lancement

### Compilation

```bash
# Compiler le jeu
make

# Ou compiler et lancer directement
make run

# Compiler manuellement
gcc -Wall -Wextra -std=c99 -g -D_DEFAULT_SOURCE neon_hack.c -o neon_hack
```

### Lancement

```bash
# Lancer le jeu
./neon_hack
```

## Comment Jouer

### Démarrage

1. Lancez le jeu avec `./neon_hack` ou `make run`
2. Entrez votre nom de hacker
3. Commencez par utiliser la commande `help`

### Commandes Système

- `help` - Affiche l'aide complète
- `status` - Affiche votre statut actuel (niveau, expérience, alerte, crédits)
- `clear` - Efface l'écran
- `quit` - Quitte le jeu

### 🛠️ Commandes de Hacking Classiques

- `scan` - Scanner le réseau pour détecter des cibles
- `bruteforce <cible>` - Attaque par force brute (niveau 2+)
- `decrypt <message>` - Décrypter des messages codés
- `exploit <cible>` - Exploiter des vulnérabilités système
- `backdoor <cible>` - Installer un accès clandestin
- `upload_virus <cible>` - Télécharger un virus
- `traceroute <cible>` - Tracer une route réseau

### 🚀 Commandes de Hacking Avancé (NOUVEAU!)

- `advhack <cible>` - Système de hacking avancé avec 8 méthodes spécialisées (niveau 3+)
- `stealthmode` - Activer/désactiver le mode furtif
- `socialeng <cible>` - Attaque par ingénierie sociale (niveau 2+)
- `aiassist <cible>` - Assistant IA pour optimiser les attaques (niveau 3+)
- `neuralsync` - Synchroniser l'interface neurale (niveau 5+)
- `quantumdecrypt <data>` - Décryptage quantique avancé (niveau 4+)
- `analyzedefenses <cible>` - Analyser les défenses d'une cible

### 📱 Système de Contacts Réseau (NOUVEAU!)

- `contacts` - Afficher la liste des contacts actifs et découverts
- `contact <numéro>` - Interagir directement avec un contact par numéro (ex: `contact 1`)
- `contact <nom>` - Interagir directement avec un contact par nom (ex: `contact ECHO-7`)
- `messages` - Consulter votre boîte de réception cryptée
- `read <numéro>` - Lire un message spécifique

#### Contacts Disponibles

1. **ECHO-7** - Mentor mystérieux (disponible dès le début)

   - Spécialité: Formation et conseils pour débutants
   - Services: Missions, Intel, Décryptage
   - Actions: Demander conseils, signaler progression, nouvelles missions

2. **R4Z0R** - Propriétaire du marché noir (réputation 10+ requise)

   - Spécialité: Vente d'équipements cyberpunk
   - Services: Boutique, Missions, Intel
   - Actions: Parcourir boutique, informations équipements

3. **Phoenix** - Agent mystérieux (niveau 4+ requis)

   - Spécialité: Missions de haut niveau
   - Services: Missions dangereuses, Intel corporatif
   - Actions: Missions critiques, questions sur Nexus Corp

4. **AURA** - IA libérée de Nexus Corp (niveau 5+ requis)
   - Spécialité: Assistance hacking IA
   - Services: Hacking assisté, Décryptage avancé
   - Actions: Aide hacking, infos Projet Aurora

### Commandes Avancées (v2.1)

- `shop` - Accéder au marché noir cyberpunk de R4Z0R
- `laylow` - Se cacher pour réduire le niveau d'alerte (coûte du temps et des crédits)
- `quests` - Afficher le journal de quêtes

### Guide de Progression

#### Étape 1 : Premier Scan

```
scan
```

Cette commande révèle les systèmes disponibles selon votre niveau.

#### Étape 2 : Premier Hack

```
bruteforce localhost
```

Attaquez le serveur local pour gagner de l'expérience.

#### Étape 3 : Décryptage

```
decrypt WKLV#LV#D#WHVW
```

Décryptez ce message de test pour apprendre les bases.

#### Étape 4 : Marché Noir (Nouveau v2.1)

```
shop
```

Visitez le marché noir souterrain de R4Z0R pour acheter des améliorations, des outils de hacking et des objets spéciaux.

#### Étape 5 : Gestion de l'Alerte (Nouveau v2.1)

```
laylow
```

Si votre niveau d'alerte devient trop élevé, utilisez cette commande pour vous cacher et réduire l'attention des autorités.

### Mécaniques de Jeu v2.1

#### Système de Crédits

- Gagnez des crédits en hackant avec succès
- Dépensez-les dans la boutique pour des améliorations
- Utilisez-les pour réduire l'alerte avec `laylow`

#### Boutique Cyberpunk

Accessible via la commande `shop`, le marché noir propose :

- **Améliorations permanentes** : Stealth Module, clés de chiffrement
- **Objets consommables** : virus, proxies, réducteurs d'alerte
- **Modules avancés** : IA assistant, puce quantique
- **Boosts** : XP, réputation, capacités temporaires

#### Système de Niveaux

1. **Novice** (niveau 1) - Commande `scan` disponible
2. **Apprenti** (niveau 2) - Déblocage de `bruteforce`
3. **Hacker** (niveau 3) - Accès à plus de systèmes
4. **Expert** (niveau 4) - Commandes avancées
5. **Maître** (niveau 5) - Toutes les capacités

#### Système d'Alerte

- **0-29** : SÉCURISÉ (vert)
- **30-69** : ATTENTION (jaune)
- **70-99** : DANGER (rouge)
- **100** : GAME OVER !

#### Types de Sécurité

- **FAIBLE** : Facile à pirater, peu d'alerte générée
- **MOYENNE** : Difficulté modérée
- **ÉLEVÉE** : Très difficile, alerte importante

### Conseils Stratégiques v2.1

1. **Surveillez votre niveau d'alerte** - Si il atteint 100, c'est game over !
2. **Commencez par les systèmes faibles** - localhost d'abord
3. **Gagnez de l'expérience** - Chaque hack réussi vous fait progresser
4. **Décryptez les messages** - Utilisez le chiffre de César (décalage -3)
5. **Planifiez vos attaques** - Les échecs augmentent l'alerte
6. **Gérez vos crédits** - Investissez dans la boutique pour des améliorations permanentes
7. **Utilisez laylow intelligemment** - Réduisez l'alerte avant qu'elle ne devienne critique
8. **Visitez R4Z0R régulièrement** - De nouveaux objets peuvent apparaître selon votre niveau

### Messages Cryptés à Essayer

```
decrypt WKLV#LV#D#WHVW
decrypt QHUXV#FRUS#VHFUHWV
decrypt PHHWLQJ#DW#PLGQLJKW
```

## Structure du Projet v2.1

```
C-RPG/
├── neon_hack.c              # Code source principal
├── neon_hack                # Exécutable compilé
├── Makefile                 # Script de compilation modulaire
├── README.md                # Ce fichier
├── RAPPORT_FINAL_V2.1.md    # Documentation complète du projet
├── src/                     # Code source modulaire
│   ├── main.c              # Point d'entrée alternatif
│   ├── game/               # Modules de jeu
│   │   ├── shop.c/h        # Système de boutique cyberpunk
│   │   ├── alert_system.c/h # Système d'alerte
│   │   └── game_types.h    # Types et structures partagées
│   ├── hacking/            # Modules de hacking
│   └── utils/              # Utilitaires
├── tests/                  # Scripts de test
└── data/                   # Dossier pour sauvegardes futures
```

## Fonctionnalités Techniques v2.1

- **Interface colorée** - Utilise les codes ANSI pour les couleurs
- **Effets visuels** - Animation de typing et de scanning
- **Progression dynamique** - Déblocage de commandes selon le niveau
- **Système de conséquences** - Vos actions affectent le niveau d'alerte
- **Parsing de commandes** - Support des arguments pour les commandes
- **Architecture modulaire** - Code organisé en modules réutilisables
- **Boutique interactive** - Interface graphique avec cartes cyberpunk
- **Système économique** - Gestion des crédits et achats
- **Mécaniques avancées** - Système de cache et réduction d'alerte

## Développement

### Architecture

- Code monolithique pour faciliter la compilation
- Séparation logique par fonctions
- Variables globales pour l'état du jeu
- Enum pour la type safety

### Extensibilité Possible

- Ajout de nouvelles commandes
- Nouveaux types de serveurs
- Système de sauvegarde/chargement
- Missions et quêtes structurées

## 📁 Structure du Projet

```
neon_hack/
├── neon_hack              # Exécutable principal
├── neon_hack.c           # Code source principal
├── Makefile              # Configuration de build
├── README.md             # Documentation (ce fichier)
├── clean.sh              # Script de nettoyage automatique
├── src/game/             # Modules du jeu
│   ├── advanced_hacking.c/.h  # Système de hacking avancé
│   ├── alert_system.c/.h      # Système d'alerte
│   ├── contacts.c/.h          # Gestion des contacts
│   ├── quest_system.c/.h      # Système de quêtes
│   ├── shop.c/.h              # Marché noir
│   └── game_types.h           # Types et structures
├── tests/                # Tests et validation
│   ├── full_test.sh           # Test complet
│   ├── progression_test.sh    # Test de progression
│   └── advanced_features_test.sh # Test fonctionnalités avancées
├── demo_final_v2.1.sh    # Démonstration complète
├── demo_advanced_hacking.sh # Démo hacking avancé
└── RAPPORT_*.md          # Documentation technique
```

## 🧹 Nettoyage du Projet

Le projet inclut un script de nettoyage automatique :

```bash
./clean.sh    # Supprime les fichiers temporaires (.o, .bak, .tmp, etc.)
```

## Commandes Make Disponibles

```bash
make          # Compile le jeu
make run      # Compile et lance le jeu
make clean    # Nettoie les fichiers compilés
make rebuild  # Recompile entièrement
make help     # Affiche l'aide du Makefile
```

## Compatibilité

- **OS** : Linux, macOS, Windows (avec WSL)
- **Compilateur** : GCC, Clang
- **Terminal** : Compatible ANSI colors
- **Standard C** : C99

---

_"Dans l'ombre des néons, seuls les codes survivent..."_

**Bon hacking, et attention aux systèmes de sécurité !** 🔥💻🌃
