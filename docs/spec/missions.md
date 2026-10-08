# Spécification : le langage de missions

> Phase P1, lot P1.2. Résout la contradiction 12 du [cross-check](../design/CROSS-CHECK.md) : le prototype n'avait que 3 conditions, la bible en demandait une dizaine, des effets, 7 états de contact et des drapeaux typés. **Cette spécification s'appuie sur un spike exécuté** : le crate [`spikes/missions`](../../spikes/missions/README.md) charge, valide et fait jouer les quêtes M01 à M14 de la bible en entier, neuf contrats, les trois décisions et les cinq fins. Sa [référence du langage](../../spikes/missions/README.md) (§ 3) est la définition précise ; ce document en garde les décisions et ce qui change pour R2.

## 1. Verdict du spike

Un langage déclaratif **petit** suffit :

| Élément | Nombre | Détail |
|---|---|---|
| Objectifs | 14 variantes pour les 22 noms de la bible | `talk`, `buy`, `compromise`, `extract`, `open`, `link`, `reputation`, `use`, `site_state`, `pay`, `choice`, `heat_end_below`, `heat_peak_below`, `any_of` |
| Conditions | 9 | six feuilles (`flag`, `quest`, `contact`, `trust`, `opened`, `objective`) et `all`, `any`, `not` |
| Effets | 9 | `set_flag`, `set_contact_state`, `trust`, `grant`, `grant_tier`, `unlock`, `heat_force`, `heat_floor`, `settle_ending` |
| Drapeaux | 4 types | booléen, énumération, compteur, ensemble de bits |
| Statuts de quête | 5 | `UNAVAILABLE`, `AVAILABLE`, `ACTIVE`, `COMPLETED`, `FAILED` |
| États de contact | 7 | `available`, `busy`, `offline`, `compromised`, `hostile`, `silenced`, `dead` |
| Faits (entrée du bus) | 16 | `SiteCompromised`, `FileExtracted`, `Read`, `Decrypted`, `ItemBought`, `Talked`, `Paid`, `HeatChanged`, `DecisionMade`, `QuestAccepted`, `QuestRestarted`, `Tick`… |

**Preuves.** Le joueur optimiste rejoue le contenu pour **216 plans** (D1 × D2 × D3 × `echo_trust` × objectifs optionnels) et finit toujours M14 : pas d'impasse. Une et une seule fin par combinaison (les cinq sont atteintes) et exactement une ligne d'épilogue par contact. Propriétés `proptest` : `refresh` idempotent, un lot de faits donne le même état dans n'importe quel ordre, aucune récompense versée deux fois, un TOML hostile ne fait jamais paniquer. 13 tests négatifs de validation. Le tout tourne en quelques secondes dans la CI.

## 2. Principes (décidés)

1. **Un seul langage de conditions et d'effets** pour les quêtes, les objectifs, les décisions, les sujets de dialogue, les fins et l'épilogue. Aucun second langage, y compris pour calculer la fin de la campagne.
2. **Le langage ne contient aucun texte** : des ids, et des clés de texte *dérivées* des ids (`quest.<id>.title`, `contact.<c>.topic.<t>.q`…). Les plus de 600 clés du spike sont contrôlées égales aux clés dérivées ; en oublier une est une erreur de chargement.
3. **Le moteur ne rappelle personne** : `apply(fait)` inscrit dans un registre (ensembles, maxima, front de Pareto) ; `refresh(état)` lit le registre, déplace les statuts, verse les récompenses et renvoie des sorties. « Une quête finie en débloque une autre » est un point fixe borné, jamais une récursion.
4. **Statut posé avant le paiement**, et chaque paiement a une clé de réclamation unique (`quest-m05`, `breach-nexus-mainframe`…) : deux gardes indépendantes (I1, I8).
5. **Pas de Turing-complétude** : aucune boucle, aucune variable hors des drapeaux déclarés, conditions de profondeur ≤ 6, un effet ne déclenche pas d'effet, une exécution par effet, écritures concurrentes d'un même drapeau refusées au chargement.
6. **Déterminisme** : collections ordonnées, tableaux `[[…]]` dans l'ordre du fichier, aucun temps réel (un compteur de tours).

## 3. Règles tranchées par défaut

| Règle | Décision | Raison |
|---|---|---|
| **R-OPEN** (objectif déjà vrai à l'ouverture) | les objectifs d'**événement** (`compromise`, `talk`, `pay`, `heat_peak_below`) ne comptent que les faits postérieurs à l'ouverture ; les objectifs de **stock** (`buy`, `extract`, `open`, `link`, `reputation`, `use`, `site_state`, `choice`) lisent l'état, rétroactivement ; **aucune quête ne se conclut dans le `refresh` qui l'ouvre** | empêche l'enchaînement instantané signalé par l'audit (2.7) sans exiger de refaire un butin unique (impasse) ; l'ouverture n'est jamais escamotée |
| Statuts `AVAILABLE` / `ACTIVE` | `AVAILABLE` = proposée par son donneur, à accepter (`accept`) ; les quêtes principales s'ouvrent seules | donne enfin un rôle au statut que le C réservait et ne produisait jamais |
| `FAILED` | réservé aux quêtes `failable` (S12), **interdit aux quêtes principales** (validé au chargement) ; une quête non `failable` dont le pic est perdu attend `QuestRestarted` | un échec principal serait une impasse |
| Limite d'objectifs | **8** par quête (la bible a M01 à 7, M12 à 6) ; 4 alternatives par `any_of` ; texte d'objectif ≤ 64 colonnes | huit lignes tiennent dans le journal du palier 64×20 |
| Récompenses | symboliques (S/M/L/XL) résolues par `rewards.toml` ; verrou d'une quête par `unlock = { tier = n }` ou `{ quest = "id" }` | cross-check 4 |
| Sauvegarde | le registre est une collection ordonnée de clés texte : la sérialisation de `State` reste à écrire en R2 (le spike ne la fait pas) | R-9 |

## 4. Contradictions de la bible révélées par l'exécution

Seule l'exécution les fait apparaître ; trois sont de vraies impasses. Le spike a tranché comme indiqué (colonne « Retenu ») ; tout est modifiable dans les données.

| # | Contradiction | Retenu | Gravité |
|---|---|---|---|
| 1 | M01 #5 `REACH_TIER(2)` est inatteignable : la quête qui l'exige est celle qui accorde le palier 2 | remplacé par `talk echo7` | impasse |
| 2 | La règle A (palier 2 *après* M02) rend M02 et M03 injouables | suivre la colonne « Palier » : M01 → 2, M02 → 3, M04 → 4, M06 → 5, M10 → 6 | impasse |
| 3 | M03 `REPUTATION(50)` est inatteignable avec les seules récompenses de quêtes (35 à 45 avant M03) depuis que la réputation ne s'achète plus | une récompense `S` unique à la première brèche de chaque site ; un test montre le blocage sans elle | impasse |
| 4 | R4Z0R est la « récompense » de M03 alors qu'elle en est le premier objectif | débloquée à la fin de M01 | cohérence |
| 5 | M10 objectif 2 déjà vrai à l'ouverture (le grand livre est extrait en M04) | l'objectif ne contraint pas, il conditionne les options de D1 | information |
| 6 | S12 : seuil 60 ou 80, échéance absente | 60, et échec si M13 s'ouvre | précision |
| 7 | `HEAT_PEAK_BELOW` : depuis l'ouverture ou pic du run ? | depuis l'ouverture, `QuestRestarted` pour rejouer ; à rediscuter avec le moteur de run (G3) | précision |
| 8 | D2 « −1 de confiance » : échelon ou point ? | −10 points | précision |
| 9 | D1 est en M10, pas en M09 | suivre la bible 4.4 | cohérence |

## 5. Ce qui reste à décider (avant R2)

- **S-6 (propriétaire)** : valider le langage de missions tel quel (14 objectifs, 9 conditions, 9 effets), la règle R-OPEN et les neuf corrections du § 4.
- **S-7 (propriétaire)** : « Neon Angel redevient libre si le joueur reprend Radio Veille avant M13 » (bible 3.4) n'est pas exprimable : il faudrait un réacteur sur un fait hors quête (`on_fact`). Option A : ajouter ce bloc (une extension). Option B : retirer la règle (Angel reste `silenced` ou `free` selon un seul choix de M08). **Recommandation : B**, qui garde le langage fermé.

## 6. Ce que R2 fait de ce spike

1. **Porter** `schema`, `content`, `validate`, `state`, `eval`, `engine`, `texts`, `ids` dans `neon-engine` (≈ 3 100 lignes ; le chargeur et l'évaluateur seuls ≈ 1 900). Le moteur n'a pas d'E/S : le chargeur prend déjà des chaînes. Le `Fact` du spike devient le `Fact` du bus interne (cross-check 1).
2. Brancher les **newtypes saturants** (crédits, réputation) et les `Text` réels : les `Output` du spike deviennent des `Event`.
3. Écrire la **sérialisation de l'état** dans l'enveloppe de sauvegarde (clés texte, versionnée, validation au chargement).
4. Le **joueur optimiste** (≈ 430 lignes) devient un outil de test et de `neon-sim` (voir [`neon-sim.md`](neon-sim.md)).
5. Brancher les **budgets de largeur** (`texts::check_budgets`) sur `unicode-width` et les catalogues FR/EN (cross-check 14, [écran d'intrusion](run-screen.md) § 6.3).
6. **Supprimer `spikes/missions`** une fois porté : c'est une preuve de concept, pas une dépendance.

## 7. Limites

L'indépendance à l'ordre vaut au sein d'un lot de faits (`apply` puis un seul `refresh`) ; entre deux lots l'ordre compte par construction (« après l'ouverture »). Les quêtes S01-S05, S09 et S15 ne sont pas écrites (aucune décision ni avantage ne dépend d'elles, sauf l'avantage `blind_spot`, qui ne change aucune fin). Le marché fermé à Notoriété 80 et le service de Ghost Walker relèvent du hub, pas du langage.
