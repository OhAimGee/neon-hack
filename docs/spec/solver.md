# Spécification : le solveur d'intrusion

> Phase P1, lot P1.5. Résout le point 11 du [cross-check](../design/CROSS-CHECK.md) (le solveur est cité comme outil de test, mais le mode histoire et la campagne en `AutoResolve` exigent un résolveur **à l'exécution** ; rien ne le définissait) et lève le risque n° 1 de la [feuille de route](../ROADMAP.md) (le moteur d'intrusion n'avait qu'un jouet de 6 nœuds sans patrouille). **Cette spécification s'appuie sur un spike exécuté** : [`spikes/solver`](../../spikes/solver/README.md) (modèle complet avec patrouilles, quatre familles d'ICE, huit nœuds, A\* avec dominance, force brute indépendante, mesures). Ses tableaux détaillés font foi ; ce document en garde les décisions.

## 1. Verdict

| Question | Réponse | Mesure |
|---|---|---|
| Un solveur **exact** peut-il donner l'optimum d'un run à 8 nœuds avec la panoplie complète du palier 5 ? | **Non** | 4 missions sur 12 dépassent 1 000 000 d'expansions (7 à 14 s, 150 Mo) ; `boss_8` en demande 813 000 (9 s) |
| Peut-on **prouver qu'une intrusion est soluble** avec le kit minimal du palier (I2) ? | **Oui** | 0,04 à 0,8 ms, sauf 56 ms pour `boss_8` ; les preuves d'impossibilité : 6 expansions (blocage logique) à 0,8 s (plafond de Trace) |
| Peut-on calculer l'optimum avec le **kit minimal** (donc la marge de difficulté, I21) ? | **Oui** | 2 à 90 ms en général, 292 ms au pire |
| Optimum complet des missions à 6 nœuds ou moins | **Oui** | 0,6 à 320 ms |
| La **prévision** a-t-elle besoin du solveur ? | **Non** | `forecast` applique le plan courant puis `end` par le même `step` que la résolution (I10), en microsecondes |
| Re-résoudre à chaque tour | bon marché | le pire coût est la première résolution (`cipher_6` : 230 ms au tour 0, puis 50, 8, 2,6 et 0,2 ms) |

Validation du solveur : une force brute indépendante donne le même optimum et le même nombre de tours sur quatre missions (jusqu'à 169 053 états) ; dix configurations du solveur s'accordent sur trois missions ; les filtres d'actions canoniques ne perdent rien ; la propriété « prévision = résolution » est testée sur environ 1 000 plans aléatoires.

## 2. Décision : le rôle du solveur

> **Le solveur exact est un outil de test et de CI. À l'exécution, le moteur n'embarque qu'une version bornée avec repli.**

Quatre usages, quatre moteurs de calcul :

| Usage | Calcul | Où |
|---|---|---|
| **Prévision** (la ligne « Prévision », la projection) | `forecast` = un `step` ; aucun solveur | exécution |
| **Preuve de solvabilité** d'une intrusion (I2), à la validation du contenu et en CI | recherche exacte à sortie anticipée, kit minimal du palier | tests, CI, `neon-sim` |
| **Marge de difficulté** (I21) | optimum exact avec le **kit minimal** (pas le kit complet : § 4) | tests, CI, `neon-sim` |
| **`AutoResolve`** (mode histoire, campagne avant le moteur tactique) | chaîne bornée : (1) exact borné à **N = 5 000 expansions**, (2) sinon repli `anytime` (20 000 expansions à sortie anticipée, 15 à 35 ms), (3) sinon **Trace nominale par palier** (valeur de contenu), marquée « approximatif » dans la sauvegarde | exécution |

**Conséquence sur G1/G8.** Les dossiers disaient « `AutoResolve` = application du plan **optimal** du solveur ». Ce n'est tenable que pour les petites intrusions (9 cas sur 23 mesurés sont décidés exactement à N = 5 000 ; toutes les missions de palier 5 à kit complet tombent dans le repli). `AutoResolve` devient « le meilleur plan que le solveur borné trouve, jamais pire que la Trace nominale du palier » : le mode histoire reste jouable de bout en bout, sans promettre l'optimum. Le rapport d'intrusion dit « automatique » (run-screen § 3.3) ; il dit « approximatif » quand l'étape 3 a servi.

## 3. Mise en œuvre retenue (pour R4)

- **Représentation.** Clé d'état fixe `u128` (84 bits utilisés) ; l'information réelle va de 23 à 56 bits selon la mission, donc une clé `u64` suffira quand le contenu sera figé. Arène + table à indices `u32` avec hachage fixe de type Fx (≈ 60 lignes) : 1,2 à 1,6 fois plus rapide que `BTreeMap` et deux fois plus compacte, déterministe (aucune graine). `BinaryHeap` standard. Environ 35 octets par état : un million d'états ≈ 50 Mo. **Pas de `HashMap`** (l'ordre d'itération), conformément à `clippy.toml`.
- **Algorithme.** A\* ordonné par `Trace − 10 × charges de Spoof + h` (le Spoof est la seule arête négative : un Dijkstra simple sur la Trace serait faux), `h` = minorant admissible (bruit minimal restant vers chaque objectif, ambiante des tours supplémentaires, crédit d'un Ghost), **dominance** (même case, mêmes cycles, phase, butin et drapeaux ; Trace plus haute et ressources plus basses ⇒ écarté), départage « le plus proche du but », filtres d'actions canoniques (Cloak en dernier, Ghost en premier, pas de Spoof à Trace 0), sans perte. Gain mesuré : A\* ÷ 2 à 12, dominance ÷ 3 à 5 en plus, départage 20 à 40 %. **À ne pas retenir** : la fusion en fin de tour (économise la mémoire, perd en temps), le `beam` (instable) et le `greedy` (échoue une fois sur deux).
- **Bornes.** N = 5 000 expansions pour l'exécution (12 à 70 ms selon la taille de la panoplie ; 20 000 permis jusqu'à 8 programmes) ; en CI et dans `neon-sim`, borne élevée en release. En debug le calcul est 5 à 10 fois plus lent : compiler `neon-engine` avec `opt-level = 3` dans le profil de test, ou utiliser N = 1 000 dans les tests unitaires.
- **Le repli `anytime`** trouve toujours un plan valide pour les kits complets, de qualité inégale (Trace 9 à 61 là où l'optimum est ≤ 7) ; il échoue sur `boss_8` avec le kit minimal (marge de 6), qui demande la recherche exacte à ≈ 40 000 expansions (90 ms) : la validation du contenu l'utilise, pas l'exécution.
- **Taille.** ≈ 500 à 800 lignes dans `neon-engine::run::solve` (audit § 5, système 5), plus ≈ 150 lignes de repli.

## 4. Ce que le spike a révélé sur le jeu lui-même

Ces constats **touchent le design**, pas seulement l'implémentation :

1. **Les utilitaires effacent la Trace.** Avec Spoof (−10), Cloak (annule les scans) et Ghost (annule le bruit), l'optimum de Trace tombe à **1-7 dès le palier 3** (au plus 14 pour les pires cas), même contre des ICE de force 5-6. Le jouet de l'audit (optimum 24) ne le montrait pas, faute de ces programmes. Sans réglage, la tactique devient un exercice de pose d'utilitaires, et la marge « 100 − optimum » vaut 93-99 : **inutilisable pour régler la difficulté**.
2. **Spoof est la source de l'explosion d'états** (× 2 à × 4 seul, jusqu'à × 40 avec Cloak et Ghost : 113 945 expansions contre 2 873 sans les trois sur `cipher_6`) et de la seule arête négative.
3. **Moins de nœuds n'est pas plus facile** : `cipher_6` (6 nœuds) coûte 114 000 expansions, `dual_7` (7 nœuds, 2 patrouilles) 3 700 ; le coût suit l'écart entre la borne initiale et l'optimum, pas la taille du graphe (explication plausible, non isolée).
4. **Les patrouilles coûtent peu** (phase en 4 bits pour les périodes 3 et 4, ppcm 12) : pas de raison de les simplifier ; garder un ppcm ≤ 16.
5. **Programmes dominés (I19)**, sur un échantillon de 6 missions de palier 1 à 5 : Virus et Overclock ne dégradent aucune mission quand on les retire ; tous les autres programmes sont nécessaires quelque part. La conclusion dépend des missions écrites pour le spike, pas du contenu final.

## 5. Règles proposées (sans réécrire les dossiers, R-0)

| # | Proposition | Raison |
|---|---|---|
| a | **La marge de difficulté se calcule sur le kit minimal du palier** (programmes illimités seulement), jamais sur le kit complet | marges mesurées de 6 à 58 avec le kit minimal, contre 73 à 99 avec le kit complet |
| b | **Limiter Spoof** : il ne peut pas descendre sous la Trace du début du tour, ou une seule charge par intrusion ; au minimum, le calcul de marge l'ignore | rend la Trace de nouveau décisive, retire la seule arête négative, divise la recherche |
| c | **« Grillé » dès que la Trace atteint 100 à tout instant**, pas seulement à la sortie | lecture retenue ; « à la sortie » rendrait la Trace non monotone et le solveur plus cher |
| d | **Emplacements de deck : garder 4 (palier 1) à 8 (palier 6), consommables hors deck** : au plus 7 programmes + 3 consommables | borne la combinatoire ; le pire cas du spike (10 programmes) la dépasse |
| e | **Rééquilibrer ou assumer Virus et Overclock** | I19 : aucun plan n'en a besoin sur l'échantillon |

## 6. Conséquences sur les autres spécifications

- [`run-screen.md`](run-screen.md) § 3.3 : `skip` applique « le meilleur plan du solveur borné » (non l'optimum) et le rapport dit « approximatif » à l'étape 3 de la chaîne.
- [`neon-sim.md`](neon-sim.md) : T2 se lit comme la **preuve de solvabilité avec le kit minimal** (≤ 56 ms, jamais « inconnu ») ; T4 est calculé sur le kit minimal (mesures de départ : marges de 6 à 58, à calibrer) ; T13 reprend les bornes de § 3.
- [`commands.md`](commands.md) : aucune commande ne change ; `skip` est ouverte en difficulté Histoire seulement.
- `spikes/solver` reste dans le dépôt comme preuve et comme base de portage ; il est supprimé quand `neon-engine::run::solve` existe (R4), comme `spikes/missions` en R2.

## 7. Décisions (validées par le propriétaire le 8 octobre 2026)

- **S-9** : le rôle du solveur (outil de test et de CI ; exécution bornée avec repli, `AutoResolve` « meilleur effort », § 2).
- **S-10** : la marge de difficulté se calcule sur le kit minimal du palier (§ 5 a).
- **S-11** : les règles de § 5 b à e, en particulier la limitation de Spoof. C'est la décision de design la plus lourde de cette phase : elle décide si la tactique de l'intrusion garde un enjeu au palier 3 et au-delà.

## 8. Limites

Le modèle est une lecture de l'audit (chaque écart est marqué `CHOICE` dans `spikes/solver/src/model.rs`). Absents : brouillard et `probe`, compagnon, effet inter-intrusions de Backdoor, butin facultatif. Les missions sont écrites pour mesurer, ni équilibrées ni validées. Une seule machine virtualisée (Xeon 2,8 GHz, 4 cœurs), aucun « poste modeste » réel ; l'admissibilité de l'heuristique n'est testée que sur des états aléatoires de deux petites missions ; avec A\* ou la dominance, le départage par nombre de tours n'est plus garanti minimal (la Trace reste exacte) ; le repli `anytime` ignore les plans dont le pic de Trace dépasse la valeur finale visée. Non essayés : clé `u64`, IDA\*, recherche bidirectionnelle, base de motifs.
