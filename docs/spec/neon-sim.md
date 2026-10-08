# Spécification : `neon-sim`, le banc d'équilibrage

> Phase P1, lot P1.4. Comble le trou « `neon-sim` : aucun bot, métrique, seuil d'acceptation, durée de partie visée ni emplacement CI défini » du [cross-check](../design/CROSS-CHECK.md) (§ 2). Le crate existe en talon depuis R0 ; il se construit en R6. Ce document dit **ce qu'il mesure, comment, et avec quels seuils**, pour que R6 soit un travail d'ajustement et non de conception. Les invariants I1-I22 viennent de l'[audit](../design/game-systems-audit.md) § 4.

## 1. Rôle

`neon-sim` joue le **vrai moteur** avec des joueurs automatiques (*bots*) et répond à trois questions :

1. **Le jeu est-il faisable ?** Aucune impasse, aucune intrusion insoluble, toute difficulté terminable (invariants I2, I17).
2. **Est-il équilibré ?** Économie, difficulté monotone, programmes utiles, anti-farm (I1, I19-I21).
3. **Dure-t-il ce qu'on a promis ?** Durée de campagne, taux d'échec, fréquence de « grillé ».

Il ne **modifie** jamais les données : il produit un rapport ; un humain règle les nombres (`data/world/*.toml`), puis relance.

## 2. Principes

- **Mêmes entrées qu'un humain.** Un bot joue par le contrat public (`Game::handle(Input)`, `Game::view()`, `Prompt`) : aucun accès privilégié à l'état. Seul le *bot solveur* appelle `run::solve`, qui est une fonction publique du moteur (même chose que la prévision et `AutoResolve`).
- **Déterministe.** Une graine de partie (celle du jeu) et une graine de bot (flux séparé, PCG32 du moteur) : même paire ⇒ mêmes octets de rapport. Aucune horloge, aucun `HashMap` dans les calculs ; les rapports sont triés.
- **Pas de dépendance cachée.** `neon-sim` peut dépendre de `clap` et `serde_json` (le JSON est réservé aux rapports : décision D2 d'architecture) ; le moteur, non.
- **Les seuils sont des données** (`data/sim/targets.toml`), validés avec le propriétaire en R6 : le code ne contient aucun chiffre d'équilibre.

## 3. Les bots

Tous partagent un cœur « joueur de campagne » (hub : quêtes, contacts, boutique, deck ; intrusion : `hack` et les commandes de `commands.md` § 2.3) ; ils diffèrent par la **politique**.

| Bot | Politique | Sert à mesurer |
|---|---|---|
| `optimist` | fait tout ce qui est faisable, dans l'ordre de la campagne, toutes combinaisons D1 × D2 × D3 (étend le joueur optimiste du spike [missions](missions.md)) ; en intrusion, applique le plan optimal du solveur | absence d'impasse (I2), couverture des fins, durée minimale |
| `prudent` | accepte les contrats utiles, équipe le deck recommandé, joue les intrusions avec la prévision (esquive les scans, masque à propos), `undo` si disponible | taux de réussite et de « grillé » d'un joueur raisonnable, durée typique |
| `reckless` | n'utilise que le programme le plus puissant, ne masque jamais, ignore la prévision | difficulté plancher (doit échouer en Expert) |
| `hoarder` | n'achète que des consommables, jamais d'amélioration | économie : on ne doit pas pouvoir tout acheter ni s'en passer |
| `farmer` | rejoue les intrusions grillées, retourne sur chaque site, répète chaque achat et chaque conversation | anti-farm (I1) : gain net par répétition ≤ 0 |
| `monkey` | entrées valides au hasard (lignes complétées par `complete`) et hostiles | robustesse (aucune panique, I13), partie toujours quittable |
| `story` | difficulté Histoire, `skip` à la première intrusion qui coûte plus d'un tour | mode histoire : la campagne se termine sans tactique |

Un bot est un type qui implémente `fn choose(&mut self, view: &View, prompt: &Prompt, rng: &mut Pcg32) -> Input`. Il ne voit que le texte structuré (`Event`, `View`), jamais l'état interne.

## 4. Métriques

Le rapport (JSON, `schema_version`) contient, par couple (bot, difficulté), pour N graines :

| Famille | Métrique |
|---|---|
| Issue | taux de campagne terminée, de fin (E1a…E4), d'abandon (budget épuisé), de partie perdue (Hardcore) |
| Durée | commandes du joueur par campagne (min, p10, médiane, p90, max) ; tours d'intrusion par campagne ; **durée estimée** = commandes ÷ cadence de référence (§ 6) |
| Intrusions | nombre, taux de « grillé », tours, Trace finale (moyenne, p90), **marge** (plafond − Trace minimale du solveur) par site |
| Notoriété | valeur moyenne et maximale, part des campagnes ayant atteint chaque bande |
| Économie | crédits gagnés par source (missions, butin, brèches), dépensés par puits, solde final, part du catalogue acheté |
| Utilité des programmes | par programme : parties où il est équipé, utilisations, **Δ marge sans lui** (invariant I19) |
| Solveur | états étendus, temps, taux de repli (« inconnu »), pire cas |
| Robustesse | paniques, parties non quittables, états invalides refusés, chargement de sauvegarde `snapshot → load → snapshot` identique |

Chaque métrique a une valeur par graine, agrégée à la fin ; les **pires graines** sont listées pour être rejouées (`--seed`).

## 5. Seuils d'acceptation

Les nombres sont des **hypothèses de départ** `[H]`, issues de l'audit et de la bible, à calibrer en R6 avec le propriétaire. Ils vivent dans `data/sim/targets.toml`. Légende : **P** = barrière dure (fait échouer la CI), **O** = objectif (signalé, ne bloque pas).

| # | Seuil | Réf. | Type |
|---|---|---|---|
| T1 | `optimist` termine la campagne sur 100 % des graines, dans les 4 difficultés et pour les 216 combinaisons de décisions ; les 5 fins sont atteintes | I2 | P |
| T2 | chaque site est **soluble** par le solveur avec le kit minimal du niveau où il apparaît, en Normal ; aucun repli « inconnu » sur le contenu livré | I2, cross-check 11 | P |
| T3 | toute intrusion se termine en au plus **50 tours** sans action du joueur (Trace ambiante ≥ +2 par tour, 100 ÷ 2) ; la solution optimale d'un site fait entre 3 et 12 tours | I17 | P |
| T4 | **marge** du solveur (plafond − Trace minimale) : Normal entre 25 et 70 sur les quêtes principales (ni couteau, ni cadeau) ; Histoire ≥ Normal + 10 ; Normal ≥ Expert ≥ Hardcore, pour chaque site | I21 | P (ordre), O (bornes) |
| T5 | `prudent` en Normal : campagne terminée ≥ 95 % ; intrusions « grillées » entre 10 et 25 % ; au plus 4 « grillé » consécutifs sur un site | — | O |
| T6 | `reckless` en Expert : campagne terminée ≤ 10 % ; en Histoire : ≥ 80 % | — | O |
| T7 | **économie** : catalogue permanent total entre 1,2 et 1,5 × revenus maximaux de la campagne (ordre de grandeur 12-14 k¢ de revenus, 15-18 k¢ de catalogue) ; le revenu minimal d'un palier couvre le kit du palier suivant ; `hoarder` et `prudent` ne peuvent pas tout acheter | I20 | P |
| T8 | **anti-farm** : `farmer` ne gagne rien de plus que `optimist` (gain net par répétition ≤ 0 crédit et ≤ 0 réputation) ; aucune séquence ne dépasse les plafonds du contenu | I1 | P |
| T9 | **aucun programme dominé** : pour chaque programme, il existe un site où le retirer fait baisser strictement la marge ; aucun programme ne bat un autre sur tous les sites | I19 | P |
| T10 | **durée** de campagne du `prudent` : entre 1 500 et 2 500 commandes (≈ 4 à 5 h à la cadence de référence) ; jamais plus de 30 % du temps en intrusion | audit 5 | O |
| T11 | **Notoriété** : `prudent` reste au plus « Surveillé » en moyenne et n'est « Chassé » que dans 10 % des campagnes ; `reckless` atteint « Chassé » dans ≥ 50 % | audit 3.7 | O |
| T12 | **robustesse** : `monkey` sur 1 000 parties ne produit ni panique, ni partie non quittable ; `snapshot → load → snapshot` identique pour tout état visité | I3-I5, I13 | P |
| T13 | **performance** : 1 000 campagnes en `AutoResolve` en moins de 60 s (release, machine de CI) ; résolution d'un site dans le pire cas en moins de 50 ms ; le rapport est identique d'une exécution à l'autre et d'un système à l'autre | I5 | P |

Un seuil « O » qui échoue produit un avertissement avec la graine la plus défavorable ; il devient « P » quand le propriétaire valide la valeur.

## 6. Cadence de référence et durée

Aucune horloge dans le moteur : la durée est **estimée** = nombre de commandes ÷ cadence de référence, fixée à **8 commandes par minute** `[H]` (lecture, réflexion, frappe). La valeur est un réglage du rapport, pas une mesure ; elle sera recalibrée avec une partie jouée par un humain (R7, protocole d'essai).

## 7. Interface et CI

```
neon-sim run   --bot prudent --difficulty normal --seeds 1..=500 --out target/sim/prudent-normal.json
neon-sim check --targets data/sim/targets.toml target/sim/*.json      # code de sortie 1 si une barrière P échoue
neon-sim solve --site corp-server-01 --difficulty normal             # plan optimal, marge, états, temps
neon-sim list-worst --report target/sim/prudent-normal.json          # graines à rejouer
```

- **Sur chaque PR** : `neon-sim check` en mode rapide (50 graines par bot, `optimist` sur les 216 combinaisons de décisions, `monkey` sur 100 parties), budget de 3 minutes, intégré au job `test` (Linux). Barrières T1-T3, T8, T9, T12.
- **Chaque nuit** (comme le `proptest` élevé) : 5 000 graines par couple, toutes les barrières, rapport JSON publié comme artefact ; un écart de plus de 5 % d'un indicateur par rapport à la veille est signalé.
- **À la demande** : `workflow_dispatch` avec les graines et les difficultés au choix, pour régler une valeur de données.

## 8. Procédure de réglage (R6)

1. Lancer la campagne complète pour tous les bots et difficultés ; lire le rapport.
2. Corriger **une famille de nombres à la fois** (prix, récompenses, force d'ICE, cycles, bruit), jamais le code.
3. Relancer ; vérifier que les barrières P restent vertes et que les objectifs O se rapprochent.
4. Quand un objectif O est atteint et validé, le promouvoir en P dans `targets.toml`.
5. Consigner dans `DECISIONS.md` chaque valeur arrêtée avec le rapport qui la justifie.

## 9. Limites et décisions

- Les bots ne reproduisent pas un humain : ils bornent le jeu (plancher, plafond, robustesse), ils ne mesurent pas l'ennui ni la lisibilité. R7 y ajoute un essai humain.
- La durée dépend de la cadence supposée (§ 6) et du volume de texte lu, que le moteur ne connaît pas.
- Le *bot solveur* dépend de la borne du solveur et de son repli ; tant que le second spike n'est pas intégré, T2-T4 et T13 sont des **cibles**, pas des mesures.
- **S-8** (propriétaire, [`DECISIONS.md`](../design/DECISIONS.md) § 5) : valider le principe des seuils en données, les barrières P/O du § 5 comme point de départ, et la cadence de 8 commandes par minute.
