# Spike P1 : second solveur de runs (patrouilles, familles d'ICE, 8 nœuds)

Crate `neon-spike-solver` (jetable, `publish = false`, std seule, aucune dépendance, édition 2024,
MSRV 1.88, `#![forbid(unsafe_code)]`, table de lints propre). Elle répond à la question du
risque n° 1 de la feuille de route (`docs/ROADMAP.md`, P1) et du point 11 de
`docs/design/CROSS-CHECK.md` : **un solveur exact et déterministe peut-il vivre dans
`neon-engine` à l'exécution** (preuve de solvabilité, prévision, `AutoResolve`, marge de
difficulté) sur un modèle de run réaliste ? Le premier spike (audit § 3.3) ne couvrait qu'un jouet
de 6 nœuds sans patrouille ni famille d'ICE. Les dossiers de conception ne sont pas modifiés (R-0).

## Verdict en cinq lignes

1. **Non, pas tel quel, pour l'optimum d'un run à 8 nœuds avec la panoplie complète du palier 5** :
   1 000 000 d'expansions (7 à 14 s en release, 150 MB) ne suffisent pas pour 4 des 12 missions
   (3 missions à 8 nœuds, plus la variante à 11 programmes), et 1 autre (`boss_8`) en demande 813 000 (9 s).
2. **Oui pour tout le reste** : preuve « solvable avec la panoplie minimale » (I2) en **moins d'1 ms**
   (sauf 56 ms pour `boss_8`), optimum avec la panoplie minimale en **≤ 0,3 s** sur le pire cas
   (typiquement 2 à 90 ms), optimum complet des missions à ≤ 6 nœuds en **0,6 à 320 ms**.
3. Ce qui fait exploser la recherche n'est ni le nombre de nœuds ni les patrouilles, mais les
   **utilitaires qui annulent de la Trace** (Spoof, Cloak, Ghost) : l'optimum tombe à 0-7 et la
   recherche doit épuiser tous les états dont la borne est inférieure (×3 à ×40, tableau « utilitaires »).
4. Recommandation : le solveur exact est un **outil de test et de CI** (release, borne en
   expansions) ; à l'exécution, la prévision n'a pas besoin de lui (`forecast` = un `step`), et
   `AutoResolve` utilise **un solveur borné (N = 5 000 expansions) puis un repli « anytime » (20 000 expansions à sortie anticipée)**.
5. Le modèle comporte au moins un mécanisme à simplifier (Spoof) et une métrique à corriger (la
   marge de difficulté doit se calculer sur la panoplie minimale) : voir « Recommandations ».

## Modèle (valeurs de départ : audit § 3.3 ; chaque écart est marqué `CHOICE` dans `src/model.rs`)

Graphe de 4 à 8 nœuds, arêtes connues (**brouillard non modélisé** : pas de `probe` ni de
compagnon ; le solveur est omniscient). ICE de famille Réseau / Chiffrement / Humain / Gardien IA,
force 1-6, progression persistante. Cycles 3 / 4 / 5 par palier (+1 en mode Histoire). Programmes de
la table de l'audit, règle de famille (hors famille : puissance 1). Trace 0-100, ambiante +2 par
tour, bruit des programmes, scans de 5 par Sentinelle, bandes 30 / 50 / 70 / 100, préréglages de
difficulté (gain ×0,5 / ×1 / ×1,25 / ×1,5, cycles bonus). Patrouilles : route cyclique annoncée,
un pas par tour, déterministe ; période 3 et 4 simultanées (ppcm 12) dans les pires cas.

| Choix du spike (écart ou précision) | Pourquoi |
|---|---|
| Arrondi du gain de Trace au supérieur (`ceil(brut × k / 4)`) | l'audit ne le dit pas ; un bruit de 1 ne doit jamais disparaître |
| Backdoor = brèche faible (puissance 1, bruit 1, illimitée) | son effet « le prochain run démarre ici » traverse les runs |
| Cloak : annule **tous** les scans de la fin du tour courant | lecture de « annule le prochain scan » avec l'exemple de tour de l'audit (deux Sentinelles annulées par un seul Cloak) |
| Ghost : annule le bruit des actions **suivantes** du tour (ni l'ambiante, ni les scans) | un remboursement rétroactif rendrait la Trace non monotone dans le tour |
| Overclock : +2 cycles, bruit 3 (annulable par Ghost) | table de l'audit |
| Virus = brèche (puissance 2 sur Gardien IA) **et** neutralise 2 tours les Sentinelles posées sur le nœud visé | l'audit ne dit pas comment on vise une Sentinelle |
| Sentinelle statique = processus d'un nœud Gardien IA : elle s'éteint quand cette ICE est franchie ; une patrouille n'a pas d'ancre | rend le Gardien IA utile et les patrouilles purement temporelles |
| Fin de tour : les patrouilles avancent, **puis** elles scannent depuis leur nouvelle position (c'est ce que montre la prévision), la règle « distance 2 si Trace ≥ 50 » lit la Trace d'avant les scans, puis l'ambiante | indépendant de l'ordre des Sentinelles |
| Butin : `loot` coûte 1 cycle ; les objectifs sont obligatoires ; l'Intel (Ingénierie sociale) est un butin ; `jackout` est gratuit, immédiat, possible partout, sans scan | simple et vérifiable ; la sortie immédiate est la lecture la plus favorable |
| « Grillé » dès que la Trace atteint le plafond **à tout instant**, même si un Spoof l'aurait ramenée | règle du jeu telle quelle (le plafond d'une mission est 100 par défaut, paramétrable) |
| Spoof est toujours à charges finies (2) | le minorant du solveur en dépend |
| Actions canoniques : Cloak joué = il ne reste que `end` ; Ghost seulement en première action du tour ; pas de Spoof à Trace 0 | élagage **sans perte**, vérifié contre la force brute (voir plus bas) |

Missions (`src/missions.rs`, constructeurs Rust) : `easy_4`, `audit_6` (le jouet de l'audit
reconstruit), `patrol_5`, `cipher_6` (3 familles, Intel à piller, patrouille de période 4),
`dual_7` (2 patrouilles de périodes 3 et 4 + gardien, Virus, Quantique), `boss_8` (trois couches),
`worst_8` (Normal et Histoire : 8 nœuds, 4 familles, 2 patrouilles de périodes 3 et 4 + gardien,
10 programmes dont 8 à charges, 5 cycles, 2 objectifs + Intel), `overloaded_8` (pire cas, ICE +2),
`worst_8_kit11` (avec Backdoor), `unsolvable_deadlock` (impossible par blocage logique) et
`unsolvable_trace` (impossible par plafond de Trace). Chacune existe aussi avec la **panoplie
minimale du palier** (`minimal_kit` : programmes illimités seulement).

## Solveur

* **Coût** : Trace finale puis nombre de tours. La Trace fait partie de l'état, donc le coût d'un
  état ne dépend pas du chemin. **Spoof (−10) est une arête négative** : un Dijkstra de coût « Trace »
  est faux. Correction exacte : on ordonne par `f = Trace − 10 × charges de Spoof + h` (cette
  quantité ne décroît jamais le long d'une transition), et un état terminal (`jackout`) est poussé avec
  sa Trace exacte ; le premier terminal sorti est optimal. `h` = minorant admissible (bruit minimal
  des ICE restant sur le chemin le moins cher vers chaque objectif, ambiante des tours
  supplémentaires, crédit d'un Ghost) ; scans et surcoût de la bande 70 ignorés.
* **État** : `State` déplié pour les règles, clé **`u128` à champs fixes** (84 bits utilisés) pour la
  recherche. Information réelle par mission en base mixte : 23 à 56 bits, donc **une clé `u64`
  suffirait pour tout le jeu de missions** (non réalisé).
* **Mémoire** : arène (clé 16 o, parent 4, tours 2, action 1, fermé 1) + table de visités.
  Mesure : **32 à 38 o par état** (table maison), ≈ 2× plus avec `BTreeMap` (estimation).
* **Conteneurs déterministes** : `BTreeMap` contre table à adressage ouvert (4 o par case, indices dans
  l'arène, hachage de type Fx à constantes fixes, sans graine). La table maison est 1,2 à 1,6× plus
  rapide et deux fois plus compacte.
* **Variantes** : tout-ou-rien « est-ce solvable sous le plafond ? » avec sortie anticipée (priorité
  = distance au but) ; repli `anytime_plan` ; repli `beam_plan` ; repli `greedy_plan`.
* **Validations** : (a) force brute indépendante (`brute.rs` : n'utilise que `step`, essaie toute
  action concevable, états `State` entiers dans des `BTreeSet`) : même optimum **et même nombre de
  tours** sur `tiny_4`, `tiny_ghost`, `easy_4`, `audit_6` (169 053 états) ; (b) 10 configurations du
  solveur donnent le même optimum ; (c) les filtres d'actions canoniques ne changent rien ; (d)
  admissibilité de `h` testée sur des états aléatoires ; (e) propriété I10 : `forecast` et résolution
  appellent le même `step_with` ; test à PCG32 sur ≈ 1 000 plans aléatoires de 4 missions, plus un
  recalcul indépendant du total des scans ; (f) cohérence : le coût restant recalculé à chaque tour le
  long du plan optimal est constant (c'est ce qui rend la prévision par re-résolution fiable).

### Brute force contre solveur (`measure check`)

| mission | optimum force brute | tours | états force brute | optimum solveur | tours | accord |
|---|---|---|---|---|---|---|
| tiny_4 | 19 | 4 | 30 890 | 19 | 4 | oui |
| tiny_ghost | 14 | 2 | 3 639 | 14 | 2 | oui |
| easy_4 | 9 | 2 | 3 494 | 9 | 2 | oui |
| audit_6 (ICE d'origine) | 20 | 3 | 169 053 | 20 | 3 | oui |

Repère : le premier spike exhaustif comptait 3 071 546 états pour un jouet voisin ; ici la même
mission (`audit_6`, solveur de base sans heuristique) passe par 13 478 expansions en 7 ms.

## Résultats

Machine : Xeon 2,8 GHz, 4 cœurs partagés, `--release` du dépôt (`overflow-checks = true`).
Recherche exacte : états individuels, A\*, dominance, table maison, plafond 1 000 000 d'expansions.
« Marge » = plafond de Trace (100) − optimum. `UNKNOWN [a..b]` : `a` est un **minorant prouvé**, `b` un
plan réel trouvé par un repli (`?` = aucun trouvé). Les temps sont ceux d'une exécution seule.

| mission | nœuds | patrouilles | programmes (à charges) | cycles | expansions | états stockés | temps | mémoire max | o/état | optimum | tours | plan | marge |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| easy_4 | 4 | 0 | 3 (1) | 3 | 35 | 127 | 0,2 ms | 0 MB | 56 | 15 | 3 | 16 | 85 |
| audit_6 | 6 | 0 | 3 (2) | 3 | 436 | 974 | 0,6 ms | 0 MB | 32 | 27 | 4 | 16 | 73 |
| patrol_5 | 5 | 1 | 6 (3) | 3 | 8 244 | 29 379 | 11 ms | 1,4 MB | 33 | 13 | 5 | 23 | 87 |
| cipher_6 | 6 | 1 | 8 (5) | 4 | 113 945 | 312 319 | 316 ms | 20 MB | 37 | 7 | 5 | 27 | 93 |
| dual_7 (Normal) | 7 | 2 | 9 (6) | 4 | 3 739 | 20 650 | 8 ms | 1,2 MB | 37 | 3 | 4 | 22 | 97 |
| boss_8 | 8 | 1 | 10 (8) | 5 | 813 677 | 2 326 348 | 9,0 s | 148 MB | 38 | 1 | 4 | 28 | 99 |
| worst_8 Normal | 8 | 2 | 10 (8) | 5 | > 1 000 000 | 2 892 370 | 8,1 s | 148 MB | 36 | UNKNOWN [0..?] | - | - | ? |
| worst_8 Histoire | 8 | 2 | 10 (8) | 6 | > 1 000 000 | 2 765 924 | 7,4 s | 156 MB | 36 | UNKNOWN [0..3] | - | - | 97..100 |
| overloaded_8 | 8 | 2 | 10 (8) | 5 | > 1 000 000 | 3 164 327 | 14,0 s | 155 MB | 35 | UNKNOWN [4..?] | - | - | ? |
| worst_8 à 11 programmes | 8 | 2 | 11 (8) | 5 | > 1 000 000 | 2 991 485 | 7,2 s | 154 MB | 35 | UNKNOWN [0..14] | - | - | 86..100 |
| unsolvable_deadlock | 5 | 1 | 3 (3) | 4 | 6 | 11 | < 0,1 ms | 0 MB | - | **impossible** (prouvé) | - | - | - |
| unsolvable_trace | 8 | 2 | 3 (1) | 5 | 145 120 | 235 086 | 367 ms | 12,5 MB | 33 | **impossible** (prouvé) | - | - | - |

Même jeu avec la **panoplie minimale** (programmes illimités seulement) : l'optimum exact reste
abordable, et surtout **la marge devient informative** (celle de la panoplie complète est 73-99, donc
inutilisable pour régler la difficulté).

| mission (panoplie minimale) | expansions | temps | optimum | tours | marge |
|---|---|---|---|---|---|
| easy_4 | 29 | 0,1 ms | 15 | 3 | 85 |
| audit_6 | 77 | 0,1 ms | 45 | 5 | 55 |
| patrol_5 | 1 466 | 2 ms | 42 | 4 | 58 |
| cipher_6 | 1 645 | 1,7 ms | 44 | 8 | 56 |
| dual_7 | 1 218 | 2 ms | 43 | 3 | 57 |
| boss_8 | 35 316 | 90 ms | 94 | 4 | **6** |
| worst_8 Normal | 41 998 | 83 ms | 87 | 4 | **13** |
| worst_8 Histoire | 28 387 | 40 ms | 44 | 4 | 56 |
| worst_8 à 11 programmes | 83 432 | 292 ms | 53 | 7 | 47 |
| overloaded_8 / unsolvable_trace | 17 635 | 21-26 ms | impossible (prouvé) | - | - |

### Techniques comparées (même optimum, plafond 1 000 000)

| mission | technique | expansions | états | états intra-tour | écartés (dominance) | temps | mémoire |
|---|---|---|---|---|---|---|---|
| patrol_5 | base (sans filtres canoniques) | 617 133 | 889 965 | - | - | 1 141 ms | 36 MB |
| patrol_5 | base | 617 133 | 834 278 | - | - | 1 111 ms | 34 MB |
| patrol_5 | base + `BTreeMap` | 617 133 | 834 278 | - | - | 1 369 ms | 67 MB |
| patrol_5 | fusion en fin de tour | 78 804 | 301 285 | 9 307 911 | - | 2 724 ms | 18 MB |
| patrol_5 | A\* | 49 222 | 115 336 | - | - | 44 ms | 5,0 MB |
| patrol_5 | dominance seule | 55 276 | 144 714 | - | 69 712 | 151 ms | 9,5 MB |
| patrol_5 | A\* + dominance | 10 371 | 36 123 | - | 7 874 | 18 ms | 2,6 MB |
| patrol_5 | A\* + dominance + départage « proche du but » | 8 244 | 29 379 | - | 5 759 | 12 ms | 1,4 MB |
| cipher_6 | base | > 1 000 000 | - | - | - | 1 384 ms | 68 MB |
| cipher_6 | fusion en fin de tour | 188 787 | 952 432 | 35 451 541 | - | 11 520 ms | 40 MB |
| cipher_6 | A\* | 469 527 | 864 813 | - | - | 692 ms | 36 MB |
| cipher_6 | dominance seule | 497 885 | 1 056 545 | - | 319 236 | 1 902 ms | 73 MB |
| cipher_6 | A\* + dominance | 93 440 | 252 121 | - | 51 456 | 191 ms | 10,9 MB |
| cipher_6 | A\* + dominance + « proche du but » | 71 571 | 195 679 | - | 36 046 | 120 ms | 10,4 MB |
| cipher_6 | idem, `BTreeMap`, fusion en fin de tour | 4 297 | 232 550 | 1 594 615 | 8 220 | 853 ms | 19,5 MB |

Lecture : (1) **A\*** divise les expansions par 2 à 12 ; (2) la **dominance** (même case, mêmes cycles,
phase, butin et drapeaux, Trace plus haute, progressions et charges plus basses ⇒ écarté ; comparaison
en une soustraction sur des voies de bits) les divise encore par 3 à 5 (par 14 à 75 avec A\* au total) ; (3) la **fusion en fin de tour** (« symétrie » des
actions indépendantes) économise la mémoire mais **ne gagne jamais en temps** : les états intra-tour
sont recalculés pour chaque parent, alors que la table globale les partage ; (4) les filtres d'actions
canoniques ne coûtent rien et gagnent 0 à 7 % ; (5) l'ordre de départage « le plus proche du but »
gagne 20 à 40 % ; (6) la table maison est plus rapide et plus compacte que `BTreeMap`. Les chiffres
du tableau « Résultats » sont ceux de la meilleure combinaison (états individuels + A\* + dominance +
départage + table maison).

### Ce qui fait grossir la recherche (plafond 1 000 000)

| mission | variante | optimum | expansions | temps |
|---|---|---|---|---|
| patrol_5 | panoplie complète | 13 | 8 244 | 11 ms |
| patrol_5 | sans Spoof | 32 | 3 877 | 4 ms |
| patrol_5 | sans Spoof ni Cloak | 40 | 2 744 | 3,6 ms |
| patrol_5 | sans aucune Sentinelle | 6 | 1 027 | 0,9 ms |
| cipher_6 | panoplie complète | 7 | 113 945 | 227 ms |
| cipher_6 | sans Spoof | 25 | 26 120 | 24 ms |
| cipher_6 | sans Cloak | 11 | 44 694 | 83 ms |
| cipher_6 | sans Ghost | 12 | 12 251 | 12 ms |
| cipher_6 | sans Spoof, Cloak, Ghost | 35 | 2 873 | 2,7 ms |
| cipher_6 | sans Spoof, Cloak, Ghost, Exploit, Quantique, Social | 44 | 1 645 | 1,2 ms |
| cipher_6 | sans aucune Sentinelle | 2 | 10 808 | 12 ms |

Croissance avec la force des ICE (`measure scaling`) : `patrol_5` de 4 567 à 11 872 expansions
(optimum 4 → 17), `cipher_6` de 11 432 à 113 945 (0 → 7), `boss_8` de 3 902 à 937 908 (0 → 1) : le
coût suit le nombre d'états dont la borne est sous l'optimum, pas la taille du graphe.

## Évaluations demandées

**(a) Programmes dominés (I19)**, `measure domination`, 6 missions de palier 1 à 5, panoplie
complète, un programme retiré à la fois :

| programme | dans les panoplies de | se dégrade si retiré (mission : avant → après) | verdict |
|---|---|---|---|
| Brute-Force | 6 | audit_6 : 27 → impossible | nécessaire |
| Exploit | 5 | audit_6 27→43 ; patrol_5 13→15 ; cipher_6 7→8 ; boss_8 1→8 | nécessaire |
| Backdoor | 4 | easy_4 15→29 ; patrol_5 13→17 ; cipher_6 7→8 ; dual_7 3→11 | nécessaire |
| Décryptage | 4 | patrol_5 13→21 ; cipher_6 7→10 ; dual_7 3→12 ; boss_8 1→2 | nécessaire |
| Quantique | 2 | dual_7 3→11 ; boss_8 1→6 | nécessaire |
| Ingénierie sociale | 3 | cipher_6 7→11 | nécessaire |
| **Virus** | 2 | aucune | **utilisé par un plan optimal mais remplaçable** |
| Spoof | 4 | patrol_5 13→32 ; cipher_6 7→25 ; dual_7 3→23 ; boss_8 1→21 | nécessaire |
| Cloak | 6 | audit_6 27→29 ; patrol_5 13→21 ; cipher_6 7→11 ; dual_7 3→13 ; boss_8 1→6 | nécessaire |
| Ghost | 2 | cipher_6 7→12 ; boss_8 1→15 | nécessaire |
| **Overclock** | 1 | aucune | **utilisé mais remplaçable** |

Sur cet échantillon, **Virus et Overclock violent I19** (aucune mission où les retirer allonge
strictement le plan) ; Overclock n'est que dans une panoplie, donc le test ne dit presque rien de lui.
Aucun programme n'est « jamais utile ». Ces constats dépendent du jeu de missions (non validé par le
propriétaire) : le test I19 est utilisable, pas sa conclusion.

**(b) Borne dure et repli (CROSS-CHECK 11)**, `measure cap` (23 cas : 12 missions + leur panoplie
minimale, hors blocage logique) :

| borne N (expansions) | cas décidés | encore inconnus |
|---|---|---|
| 1 000 | 5 | 78 % |
| 10 000 | 10 | 57 % |
| 30 000 | 13 | 43 % |
| 100 000 | 16 | 30 % |
| 300 000 | 18 | 22 % |
| 1 000 000 | 19 | 17 % |

Coût par expansion : 0,7 à 3 µs pour les panoplies minimales (≤ 3 programmes), 1,5 à 2,7 µs jusqu'à
8 programmes et 6 nœuds, **7 à 14 µs avec 10 programmes à 8 nœuds**. Pour tenir **≈ 50 ms au pire**
il faut donc **N = 5 000 expansions** (12 ms avec une petite panoplie, 70 ms avec 10 programmes ;
N = 20 000 est permis si la panoplie a ≤ 8 programmes). À N = 5 000, **9 cas sur 23 sont décidés**
(les petites missions et les panoplies minimales) ; les 14 autres tombent dans le repli, dont
toutes les missions de palier 5 à panoplie complète. **En debug** (mesuré : ×5 à ×10, `patrol_5` 120 ms
contre 11 ms) N = 5 000 coûte 0,1 à 0,7 s : acceptable pour un test ; mieux vaut compiler le crate
moteur avec `opt-level = 3` dans le profil de test, ou garder N = 1 000 dans les tests unitaires.

Repli à N dépassé (qualité = Trace finale du plan trouvé ; optimum entre parenthèses quand il est connu) :

| mission | optimum | `greedy` (sans recherche) | `beam` 4 / 16 | **`anytime` 20 000** | temps `anytime` |
|---|---|---|---|---|---|
| easy_4 | 15 | 22 | 15 / 15 | 15 | 0,4 ms |
| audit_6 | 27 | 45 | échec / 69 | 27 | 0,7 ms |
| patrol_5 | 13 | 40 | 37 / 14 | 21 | 22 ms |
| cipher_6 | 7 | échec | 57 / 9 | 22 | 14 ms |
| dual_7 | 3 | échec | 25 / 62 | 15 | 33 ms |
| boss_8 | 1 | échec | échec | 61 | 20 ms |
| worst_8 Normal | ? | échec | échec | 35 | 21 ms |
| worst_8 Histoire | ? | échec | 3 / 3 | 9 | 15 ms |
| overloaded_8 | ? | échec | échec | 55 | 21 ms |
| worst_8 Normal, panoplie minimale | 87 | échec | échec | 87 | 34 ms |
| boss_8, panoplie minimale | 94 | échec | échec | **échec** | 19 ms |

Le repli `anytime` (recherches successives à sortie anticipée sous un plafond qui se resserre, budget
total en expansions) **trouve toujours un plan valide pour les missions à panoplie complète** en 15 à
35 ms, mais de qualité très inégale (Trace 9 à 61 quand l'optimum est ≤ 7) ; il échoue sur
`boss_8` à panoplie minimale, où la marge n'est que de 6 : c'est là que la recherche exacte à
N = 40 000 (90 ms) est nécessaire. Le `beam` est plus lent, instable (élargir peut empirer) et échoue
sur les grosses missions ; le `greedy` échoue une fois sur deux : **à ne pas retenir**.
Conduite proposée pour `AutoResolve` : (1) solveur exact borné à N ; (2) sinon `anytime` ; (3) sinon
Trace nominale par palier (valeur de contenu), avec une marque « approximatif » dans la sauvegarde.

**(c) Usage incrémental (prévision à chaque tour)**, `measure preview` : on re-résout depuis l'état
courant le long du plan optimal. Le coût restant est identique à chaque tour (cohérence vérifiée par
test) et le temps décroît vite :

| mission | tour 0 | tour 1 | tour 2 | tour 3 | tour 4 |
|---|---|---|---|---|---|
| audit_6 | 0,9 ms | 0,5 ms | 0,6 ms | 0,2 ms | 0,05 ms |
| patrol_5 | 12,6 ms | 3,8 ms | 1,5 ms | 0,4 ms | 0,06 ms |
| cipher_6 | 230 ms | 50 ms | 8 ms | 2,6 ms | 0,2 ms |
| dual_7 | 4,6 ms | 1,0 ms | 0,2 ms | 0,08 ms | 0,03 ms |

Re-résoudre à chaque tour n'est donc pas le problème (le pire est le tour 0) ; mais la « prévision »
de l'audit (la ligne « Sentinelle X scanne Y : +5 ») n'a pas besoin de solveur : `forecast` applique
le plan courant puis `End` par le **même** `step_with` que la résolution (I10), en microsecondes.

**Preuve de solvabilité (I2)**, `measure minimal` (sortie anticipée sous le plafond 100) : toutes les
missions, panoplie complète ou minimale, sont décidées en **0,04 à 0,8 ms** (11 à 361 expansions), sauf
`boss_8` à panoplie minimale : 56 ms (24 116 expansions). Les trois impossibles sont prouvés : blocage
logique en 6 expansions, `unsolvable_trace` en 0,8 s avec sa panoplie réelle (268 885 expansions) et
36 ms avec la panoplie minimale. C'est le cas coûteux annoncé, et il reste sous la seconde.

## Recommandations

**Représentation et conteneurs** : clé fixe `u128` (84 bits) pour commencer, `u64` en base mixte quand
le contenu sera figé (≤ 56 bits ici) ; arène + table à indices `u32` avec hachage fixe (≈ 60 lignes,
2× moins de mémoire que `BTreeMap`) ; `BinaryHeap` standard (déterministe). Espace ≈ 35 o par état :
1 million d'états ≈ 50 MB. Conserver : A\*, dominance, états individuels, départage « proche du
but », filtres d'actions canoniques. Ne pas retenir : fusion en fin de tour.

**Règles à simplifier ou corriger dans les dossiers (sans les réécrire, R-0)** :

1. **Spoof (−10 Trace)** : seule arête négative du modèle (elle casse le Dijkstra simple), plus gros
   multiplicateur de l'espace d'états (×2 à ×4 seul, ×40 avec Cloak et Ghost) et elle ramène l'optimum
   vers 0. Proposition : le Spoof ne s'applique qu'aux bruits **déjà accumulés** et ne peut pas
   descendre sous la Trace du début du tour, ou une seule charge par run ; au minimum, calculer la marge
   sans Spoof.
2. **Utilitaires** : Cloak, Ghost et Spoof ensemble rendent l'optimum de la panoplie complète
   quasi nul (1 à 7 sur les 3 missions de palier ≥ 3 résolues, ≤ 14 pour les pires cas). **La marge de difficulté (audit § 3.3, I21) doit se
   calculer sur la panoplie minimale** (marges de 6 à 58 mesurées) ou sur la Trace avant utilitaires.
3. **Emplacements de deck** : l'audit en donne 4 (palier 1) à 8 (palier 6). Ma panoplie de palier 5 à
   10 programmes dépasse déjà ce plafond ; si les consommables (Ghost, Spoof, Overclock) restent hors
   deck, le plafond réel est 7 programmes + 3 consommables, ce qui borne la combinatoire. À garder.
4. **Virus et Overclock** ne sont jamais nécessaires sur cet échantillon (I19) : à rééquilibrer, ou à
   garder pour le contenu (le test dépend des missions écrites).
5. **« Grillé à tout instant »** (plafond sur le pic, pas sur la valeur finale) est la lecture
   retenue ; la changer en « à la sortie » rendrait la Trace de fin de run non monotone et le solveur plus
   cher. À trancher explicitement.
6. Les **patrouilles** coûtent peu d'état (phase ≤ 4 bits pour les périodes 3 et 4, ppcm 12) : pas
   de raison de les simplifier ; garder ppcm ≤ 16.

## Ce qui est surprenant

* Les missions de palier ≥ 3 ont un optimum de Trace de 1 à 7 (au plus 14 pour les pires cas) avec la
  panoplie complète, même avec des ICE de force 5-6 : les utilitaires effacent la Trace. Le jouet de l'audit
  (optimum 24) ne le montrait pas, faute de Spoof, de Ghost et de familles.
* Moins de nœuds ≠ plus facile : `cipher_6` (6 nœuds) coûte 114 000 expansions, `dual_7` (7 nœuds,
  2 patrouilles) 3 700 : le coût suit l'écart entre la borne initiale et l'optimum, pas la taille
  (explication plausible, non isolée par une expérience).
* La fusion « propre » des actions indépendantes en fin de tour, première idée de symétrie, est la
  seule technique qui perd en temps.
* La prévision à chaque tour est bon marché ; c'est la première résolution qui coûte.

## Limites et ce qui n'a pas été vérifié

* **Fidélité** : le modèle est ma lecture de l'audit (tableau des `CHOICE`) ; pas de brouillard,
  de `probe`, de compagnon, d'effet inter-runs de Backdoor, de butin facultatif ni de plusieurs
  intrusions par mission. Les missions sont écrites pour mesurer, pas équilibrées ni validées par le propriétaire.
* **Mesures** : une seule machine (Xeon 2,8 GHz virtualisé, 4 cœurs), temps d'une exécution, aucun
  « poste modeste » réel ; la mémoire des `BTreeMap` est une estimation, celle des tables est comptée
  (capacité). Les tableaux « Résultats », « panoplie minimale », « repli » et « utilitaires » ont été
  relevés seuls ; « techniques », « dominance », « prévision » et « croissance » ont tourné en
  parallèle sur les 4 cœurs (écart observé sur les lignes communes ≤ 15 %, résultats identiques).
* **Exactitude** : le solveur est validé contre une force brute sur 4 petites missions (≤ 170 000
  états) et par accord entre 10 configurations sur 3 missions ; `h` n'est testé que sur des états
  aléatoires de deux petites missions (pas de preuve). Avec A\* ou la dominance, le **départage par
  nombre de tours n'est plus garanti minimal** (la Trace, elle, reste exacte) : `Outcome::Solved`
  porte deux drapeaux séparés, `optimal` (Trace) et `turns_minimal` (tours, vrai seulement pour la
  recherche simple sans heuristique ni dominance). Aucun contre-exemple n'a été trouvé sur 6 missions
  × 3 configurations : le drapeau est prudent, pas démontré faux. Le repli `anytime`
  ignore les plans dont le pic de Trace dépasse la valeur finale cible.
* **Non réalisé** : clé `u64` en base mixte, optimisation fine de la boucle (la mise en cache de `h` et la
  dominance en voies de bits ont déjà divisé le temps par 2 à 3), parallélisme, comparaison avec un
  autre algorithme (IDA\*, recherche bidirectionnelle, base de motifs).

## Reproduire

```text
cargo test -p neon-spike-solver                                   # < 5 s en debug
cargo run --release -p neon-spike-solver --example measure -- all # plusieurs minutes
cargo run --release -p neon-spike-solver --example measure -- table 1000000
```

Sections : `check`, `table`, `techniques`, `domination`, `cap`, `preview`, `scaling`, `utilities`,
`minimal`.
