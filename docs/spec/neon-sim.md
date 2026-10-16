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
- **Déterministe.** Une graine de partie (celle du jeu) et une graine de bot (flux séparé, PCG32 du moteur) : même paire ⇒ mêmes octets de rapport. Le rapport canonique ne contient **aucune durée mesurée** (elle varie d'une machine à l'autre) : le travail du solveur s'y lit en compteurs ; les chronométrages vont dans un rapport séparé, `bench.json`. Aucune horloge, aucun `HashMap` dans les calculs ; les rapports sont triés.
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
| Solveur | états étendus et stockés, profondeur, taux de repli (« inconnu »), pire cas : des **compteurs de travail** déterministes, jamais de durée |
| Chronométrage (rapport **à part**, non canonique) | temps du solveur par site, durée d'une campagne en `AutoResolve`, débit : écrits dans `bench.json`, hors de la comparaison d'octets |
| Robustesse | paniques, parties non quittables, états invalides refusés, chargement de sauvegarde `snapshot → load → snapshot` identique |

Chaque métrique a une valeur par graine, agrégée à la fin ; les **pires graines** sont listées pour être rejouées (`--seed`).

## 5. Seuils d'acceptation

Les nombres sont des **hypothèses de départ** `[H]`, issues de l'audit et de la bible, à calibrer en R6 avec le propriétaire. Ils vivent dans `data/sim/targets.toml`. Légende : **P** = barrière dure (fait échouer la CI), **O** = objectif (signalé, ne bloque pas).

| # | Seuil | Réf. | Type |
|---|---|---|---|
| T1 | `optimist` termine la campagne sur 100 % des graines, dans les 4 difficultés et pour les 216 combinaisons de décisions ; les 5 fins sont atteintes | I2 | P |
| T2 | chaque site est **prouvé soluble** par le solveur avec le **kit minimal** du palier où il apparaît, en Normal (preuve en ≤ 56 ms au pire sur le spike) ; aucun résultat « inconnu » sur le contenu livré | I2, cross-check 11, [solver.md](solver.md) | P |
| T3 | toute intrusion se termine en au plus **50 tours** sans action du joueur (Trace ambiante ≥ +2 par tour, 100 ÷ 2) ; la solution optimale d'un site fait entre 3 et 12 tours, **en Normal, à partir du palier 3** (R-32) | I17 | P |
| T4 | **marge** du solveur (plafond − Trace minimale) : Normal entre 25 et 70 sur les quêtes principales (ni couteau, ni cadeau), **calculée sur le kit minimal** ([solver.md](solver.md) § 5 : le spike mesure 6 à 58) ; Histoire ≥ Normal + 10 ; Normal ≥ Expert ≥ Hardcore, pour chaque site | I21 | P (ordre), O (bornes) |
| T5 | `prudent` en Normal : campagne terminée ≥ 95 % ; intrusions « grillées » entre 10 et 25 % ; au plus 4 « grillé » consécutifs sur un site | — | O |
| T6 | `reckless` en Expert : campagne terminée ≤ 10 % ; en Histoire : ≥ 80 % | — | O |
| T7 | **économie** : catalogue permanent total entre 1,2 et 1,5 × revenus maximaux de la campagne (ordre de grandeur 12-14 k¢ de revenus, 15-18 k¢ de catalogue) ; le revenu minimal d'un palier couvre le kit du palier suivant ; `hoarder` et `prudent` ne peuvent pas tout acheter (R6.7 : mesuré en quatre constats, en **objectif** tant que le propriétaire n'a pas validé les valeurs) | I20 | O |
| T8 | **anti-farm** : `farmer` ne gagne rien de plus que `optimist` (gain net par répétition ≤ 0 crédit et ≤ 0 réputation) ; aucune séquence ne dépasse les plafonds du contenu (R6.6 : mesuré en comparant les crédits et la réputation des deux sur les mêmes parties) | I1 | P |
| T9 | **aucun programme dominé** : pour chaque programme, il existe un site où le retirer fait baisser strictement la marge ; aucun programme ne bat un autre sur tous les sites (R6.9 : seule la première clause est mesurée, en **objectif** ; la seconde, comparée en gains par site, ne dit rien d'utile) | I19 | O |
| T10 | **durée** de campagne du `prudent` : entre 1 500 et 2 500 commandes (≈ 4 à 5 h à la cadence de référence) ; jamais plus de 30 % du temps en intrusion | audit 5 | O |
| T11 | **Notoriété** : `prudent` reste au plus « Surveillé » en moyenne et n'est « Chassé » que dans 10 % des campagnes ; `reckless` atteint « Chassé » dans ≥ 50 % | audit 3.7 | O |
| T12 | **robustesse** : `monkey` sur 1 000 parties ne produit ni panique, ni partie non quittable ; `snapshot → load → snapshot` identique pour tout état visité (la question en attente d'un oui ou d'un non n'est pas sauvegardée : elle est reposée) ; une sauvegarde abîmée est refusée sans toucher la partie (R6.5 : un rapport de moins de 100 parties, le contrôle rapide de la CI, ne dit rien) | I3-I5, I13 | P |
| T13 | **performance** (lue dans `bench.json`) : 1 000 campagnes en `AutoResolve` en moins de 60 s (release, machine de CI) ; résolution bornée d'un site (N = 5 000 expansions, [solver.md](solver.md) § 3) dans le pire cas en moins de 50 ms ; **déterminisme** (lu dans le rapport canonique) : identique d'une exécution à l'autre et d'un système à l'autre | I5 | O |
| T14 | **mode Histoire** (ajouté en R6.5) : `story` termine la campagne en Histoire sur ≥ 95 % des parties, et « Chassé » dans au plus 15 % des parties (R6.14) | — | O |

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

- **Sur chaque PR** : `neon-sim check` en mode rapide (`scripts/sim-gate.sh`, job `sim-gate` de la CI : `optimist` et `farmer` sur les plans représentatifs, les 16 sites au solveur, `monkey` sur 100 parties), budget de 3 minutes. Barrières T1-T4, T8, T12. T9 (la preuve de chaque programme, 3 min 30) est un objectif : il reste pour la nuit.
- **Chaque nuit** (`.github/workflows/sim-nightly.yml`, `scripts/sim-nightly.sh`, 02:43 UTC) : les six bots de campagne sur les 216 plans à toutes les difficultés, le singe sur 500 parties, les sites au solveur (objectif puis tout le butin), l'économie et les programmes, puis `neon-sim check` sur tous les seuils (P et O) ; les lignes de `check` vont dans le résumé du job, le code de sortie est celui des barrières P. Les règles n'ont pas de hasard, des graines de plus ne font varier que le reste de la partie (`SEEDS`). **Reste à faire** : publier les rapports comme artefact et signaler un écart de plus de 5 % d'un indicateur par rapport à la veille.
- **À la demande** : `workflow_dispatch` avec les graines et les difficultés au choix, pour régler une valeur de données.

## 7 bis. État de l'implémentation (R6.2 à R6.14)

Livré : `neon-sim run`, `solve` et `check`, avec les bots `optimist`, `prudent`, `reckless`, `story`, `monkey`, `farmer` et `hoarder` ; l'économie (T7, en objectif), les programmes dominés (T9, en objectif), `list-worst` reste à faire. **R6.10** : la barrière rapide tourne à chaque PR (`scripts/sim-gate.sh`, environ 45 s en version de publication, job `sim-gate` requis par `CI OK`). **R6.11** : le run de nuit (workflow `sim-nightly`, `workflow_dispatch` avec graines et plans au choix). **R6.12** : `neon-sim bench [--games N] --out bench.json` chronomètre des campagnes de l'optimiste (Normal, plans représentatifs), la résolution bornée (`auto_resolve`, tout le butin, deck complet) de chaque site à chaque difficulté, et vérifie qu'une double exécution donne les mêmes octets ; les durées sont en microsecondes, dans un fichier à part, et seul `bench.rs` lit une horloge. T13 est un objectif : **non tenu** sur le contenu livré (mille campagnes en 423 s pour 60 s permises, résolution bornée la plus lente 108 ms pour 50 ms permises, à `sentinel-hub` en Hardcore ; déterminisme tenu).

- `run [--bot optimist|prudent|reckless|story|farmer|hoarder|monkey] --difficulty all|story|normal|expert|hardcore --seeds N|A..=B --plans all|representative [--limit N] [--steps N] --out FICHIER` joue le bot (`brain` + `driver`) sur les combinaisons de décisions demandées. Avec `all`, un fichier par difficulté (suffixe ajouté au nom). Les graines servent d'amorce aux parties ; une partie qui ne peut pas continuer est une `Stuck` (jamais une panique).
- `solve [--site ID] --difficulty D [--loot objective|all] [--expansions N] --out FICHIER` mesure chaque site : par défaut (`objective`) avec `optimum` sur le kit illimité du palier (plan, marge, tours, états explorés) ; avec `--loot all`, en prenant **tout** le butin avec le deck complet du palier, comme le fait l'optimiste (les fichiers des quêtes sont sur les nœuds qui ne sont pas l'objectif) : la recherche de faisabilité seule, car l'optimum y est hors de portée. `check` en tire un second constat T2 (« gagné en prenant tout le butin »).
- `check [--targets FICHIER] RAPPORT...` lit les rapports et les seuils (par défaut ceux de `data/sim/targets.toml`, embarqués) ; code de sortie 0 (rien d'échoué), 1 (une barrière P échoue), 2 (arguments ou fichiers invalides).
- **Rapport canonique** : JSON `kind = campaign | sites | monkey | economy`, `schema = 5`, sans horloge ni flottant (les rapports se comparent octet à octet) ; les répartitions sont des rangs au plus proche (`Spread`).
- **Seuils livrés** : T1 à T4 en **P**, tous tenus par le contenu livré. T3 se lit en Normal seulement (les autres difficultés en découlent) et, comme les bornes de T4 (écart Histoire/Normal, marge en Normal), commence au palier 3 : les deux premiers sites sont le tutoriel. L'ordre des difficultés vaut pour tous les sites. Lot R6.3 : le contenu a été retouché pour les atteindre ([`DECISIONS.md`](../design/DECISIONS.md) R-32). T5, T6, T10 et T11 sont des objectifs (**O**) : les seuils sont ceux de la spec et la mesure du contenu livré dit où il en est.

**Les bots à la main (R6.4, [`DECISIONS.md`](../design/DECISIONS.md) R-33).** Les règles d'une intrusion n'ont pas de hasard : le plan du solveur ne grille jamais. `prudent` et `reckless` jouent donc **à la main**, une action à la fois, depuis l'état affiché (`neon_sim::tactics`) : ils vont au butin le plus proche, ouvrent ce qui barre la route, prennent tout et sortent. `reckless` ouvre avec le programme le plus fort contre la glace, quel que soit son bruit, n'utilise aucun utilitaire et finit le tour sans regarder la prévision. `prudent` ouvre avec le programme le moins bruyant par point de progrès, lit la prévision avant de finir un tour, masque (`cloak`) quand un scan arrive et efface (`spoof`) le bruit d'un tour bruyant ; il achète de la couverture (`laylow`) dès la bande « Surveillé » (30), alors que les autres bots n'en achètent que si une quête l'exige. Le joueur ne voit rien de plus que le jeu n'en montre ; le résultat ne dépend pas de la graine (aucun hasard dans les règles), les graines ne font varier que le reste de la partie.

Une partie a quatre issues : **terminée**, **perdue** (une intrusion grille en Hardcore), **abandonnée** (un bot à la main brûle douze fois de suite la même intrusion) ou **bloquée** (un défaut du banc, jamais un résultat). Les statistiques d'intrusion et de Notoriété comptent toutes les parties, pas seulement les terminées : un bot qui grille est ce qu'elles mesurent. Le rapport donne aussi la part de grillés, la plus longue série de grillés sur un site, la part de parties qui atteignent la bande « Chassé » (80) et la part des pas passée en intrusion.

**Mesures du contenu livré** (216 plans, graine 1, versions de R6.4) :

| Bot | Histoire | Normal | Expert | Hardcore |
|---|---|---|---|---|
| `optimist` | 216 terminées | 216 terminées | 216 terminées | 216 terminées |
| `prudent` | 216 terminées, 0 ‰ grillé | 216 terminées, 0 ‰ grillé | 216 abandonnées (`nexus-mainframe` dans les parties listées), 800 ‰ grillé | 216 perdues (`nexus-mainframe` dans les parties listées) |
| `reckless` | 216 terminées | 216 terminées, 0 ‰ grillé | 216 abandonnées (`sentinel-hub` dans les parties listées), 404 ‰ grillé | 216 perdues (`research-lab` dans les parties listées) |

- **T6 tenu** : `reckless` termine l'Histoire (1000 ‰) et pas l'Expert (0 ‰).
- **T5 non tenu** : `prudent` ne grille aucune intrusion en Normal (0 ‰ pour 100 à 250 ‰ attendus) : le Normal est facile pour un joueur qui lit la prévision. Il termine à 1000 ‰, et la série de grillés est de 0.
- **T10 non tenu** : une campagne `prudent` en Normal dure 550 lignes de commande au médian (1 500 à 2 500 attendues), et 734 ‰ des pas d'une partie se passent en intrusion au pire (300 ‰ permis).
- **T11 non tenu pour `prudent`** : même en achetant de la couverture dès 30, sa Notoriété maximale est de 65 au médian (49 permis), sans jamais atteindre « Chassé » (0 ‰ pour 100 ‰ permis) ; `reckless` est « Chassé » dans 1000 ‰ de ses parties (500 ‰ attendus).
- **Constats pour le réglage** : en Expert, un joueur sans recherche (`prudent` y compris) ne passe pas `nexus-mainframe`, alors que le solveur le gagne (marge de 42) ; en Hardcore le moindre grillage perd la partie.

**`story` et `monkey` (R6.5, [`DECISIONS.md`](../design/DECISIONS.md) R-34).**

- `story` ne joue que l'Histoire (`--difficulty` autre est refusé) : il joue chaque intrusion à la main, comme `prudent`, et tape `skip` dès que le premier tour est passé, à chaque visite. Le rapport compte la part des intrusions jouées par le jeu (`intrusions.automatic_per_mille`). **Lot R6.14 ([`DECISIONS.md`](../design/DECISIONS.md) R-40)** : le constat de R6.5 (« `skip` ne joue que l'objectif ») est réglé : `skip` prend tout le butin du site, les fichiers des quêtes inclus (`Mission::taking_everything`), et le bot n'a plus à reprendre un site à la main. La Notoriété de l'Histoire est réglée par la donnée `[heat]` de `data/world/rewards.toml` : une intrusion coûte en Notoriété une part de sa Trace (25 % comme avant, 3 % en Histoire, au moins 1). Mesure (216 plans, graine 1) : 216 parties terminées sur 216, les cinq fins, 264 lignes au médian (466 avant, sur 8 plans), 866 ‰ des intrusions jouées par `skip`, aucune grillée, Notoriété maximale médiane de 70 : « Chassé » dans 111 ‰ des parties, toutes dans les branches où la trahison force la chaleur à 85 (`heat_force` de `quests.toml`) et non par les intrusions (875 ‰ avant, sur 8 plans). **T14 tenu** (objectif).
- `monkey` (`run --bot monkey --seeds 1..=1000 [--steps N]`, 300 entrées par partie par défaut, les difficultés à tour de rôle) tape des lignes que `complete` propose avec un argument que `complete` propose, des lignes hostiles (vides, énormes, caractères de contrôle, nombres hors bornes, motifs de format), et répond au hasard aux menus, aux questions et aux textes. Il contrôle sans cesse : aucune panique (I13), le prompt d'un pas est celui du jeu, `snapshot → load → snapshot` donne le même texte (la question en attente d'un oui ou d'un non n'est pas sauvegardée : elle est reposée), une sauvegarde abîmée (coupée, un caractère de moins, changé, du texte étranger) est refusée sans toucher la partie, ou bien charge un jeu qui tient la même promesse ; et, à la fin, la partie se quitte (`quit`, oui, hors des menus). Le rapport (`kind = monkey`) compte parties, entrées, part passée en intrusion, paniques, parties non quittables, défauts de sauvegarde et de contrat, sauvegardes abîmées refusées ou prises, et liste les pires parties avec les dernières entrées pour les rejouer. **T12 en P** : aucun défaut sur au moins 100 parties.
- **Le singe a trouvé un défaut** (17 parties sur 200 au premier essai) : `clean_handle` coupait le pseudo à 20 caractères après avoir retiré les blancs, et pouvait laisser un espace final ; la sauvegarde, qui refuse un pseudo qui n'est pas propre, ne rechargeait plus la partie (« the handle is not a clean handle »). Corrigé dans le moteur (la coupe est retaillée) avec un test de régression. Ensuite : 1 000 parties de 300 entrées (9 s) et 400 de 1 500 entrées (16 s), aucun défaut.

**`farmer` et T8 (R6.6, [`DECISIONS.md`](../design/DECISIONS.md) R-35).** `farmer` suit le plan de l'optimiste, et **renvoie à l'identique** ce que le plan a envoyé : pour chaque commande jouée (la ligne et les réponses aux questions et aux menus qu'elle a causées, dédoublonnées), il la renvoie deux fois (`FARM_TIMES`). Une **ronde** a lieu à chaque quête terminée depuis la dernière : conversations (`talk`), lectures (`read`, `archives`), déchiffrements (`decrypt`) et liens (`link`) ; la dernière ronde, quand la campagne est finie, y ajoute `hack` (les sites revisités, intrusion jouée par le plan du solveur), `buy` et `laylow`. Les achats et la couverture ne sont rejoués qu'à la fin : en cours de campagne ils dépenseraient ce que le plan prévoit. Une première version qui tapait toutes les commandes offertes par `complete` faisait des premières fois (une conversation qui termine une quête, une quête facultative que le plan laisse) et gagnait plus que l'optimiste : ce n'était pas une répétition ; rejouer le même envoi l'est. Le rapport donne le nombre de lignes renvoyées par partie (`repeated`) et, pour tous les bots, la réputation gagnée (`reputation`, schéma 4).

- **T8 en P** : à chaque difficulté jouée par les deux, les crédits et la réputation du `farmer` ne dépassent ceux de l'`optimist` à aucun rang (min, p10, médiane, p90, max), sur les mêmes parties ; et le `farmer` a bien renvoyé au moins une ligne (`min_repeated`), sinon la mesure ne dit rien. Les gains d'une répétition sont à ce compte : un trou d'anti-farm fait dépasser l'optimiste.
- **Mesure du contenu livré** (216 plans, graine 1, les quatre difficultés) : le `farmer` termine 864 parties sur 864, renvoie au moins 1 186 lignes par partie (environ 1 300 au médian), refait deux fois plus d'intrusions que l'optimiste (10 900 contre 5 000 environ en Normal), et finit **avec les mêmes crédits (15 155 au médian) et la même réputation (550 en Histoire, 575 ailleurs)** : **T8 tenu**. L'anti-farm est structurel (« toute source bornée par un état ou un jalon à usage unique », [architecture](../design/architecture-rust.md)) et la mesure le confirme sur les conversations, les lectures, les déchiffrements, les liens, les sites revisités, les achats et la couverture.
- Limite connue : la confiance des contacts (`per_talk`) n'est pas mesurée, elle ne paie ni crédits ni réputation.

**`hoarder`, `economy` et T7 (R6.7, [`DECISIONS.md`](../design/DECISIONS.md) R-36).**

- `hoarder` suit le plan de l'optimiste mais n'achète **rien** à l'étal (la couverture de `laylow`, consommable, reste permise). Quand il n'a plus rien d'autre à faire, la partie est **bloquée** (`blocked`, quatrième issue d'une partie après terminée, perdue et abandonnée) par le premier objet qu'une quête ouverte demande de posséder (« blocked by stealth_module »). Le rapport compte les parties bloquées (schéma 5).
- `neon-sim economy [--out FICHIER]` lit **dans les données** ce que le contenu offre et demande (`kind = economy`) : l'étal (objet, palier, prix, quêtes qui le demandent), les services de `laylow`, et pour chaque palier le prix de l'objet phare `R(P)` et les crédits que paient les quêtes principales et toutes les quêtes du palier (borne haute : les branches exclusives y sont toutes). Les crédits de butin d'intrusion n'y sont pas : ceux de la campagne se lisent dans les rapports de campagne.
- **T7 en objectif (O)**, en quatre constats : (1) le prix de l'étal en pourcentage de ce qu'une campagne rapporte au plus (rapport de l'`optimist`) dans 120 à 150 % ; (2) l'étal coûte plus que ce qu'une campagne rapporte au moins (« on ne peut pas tout acheter ») ; (3) les quêtes principales de chaque palier paient l'objet du palier suivant ; (4) le `hoarder` ne finit aucune campagne (« on ne peut pas s'en passer »). Les valeurs sont celles de la spec ; il faut le rapport économie avec un rapport de l'`optimist` (les revenus ne dépendent pas de la difficulté) et, pour (4), un rapport du `hoarder`.
- **Mesure du contenu livré** (216 plans, Normal) : l'étal compte **5 objets pour 1 730 ¢** (80, 150, 200, 500, 800) ; une campagne rapporte entre 14 755 et 19 405 ¢ ; trois services de couverture (25, 70 et 160 ¢). **(1) non tenu** : l'étal vaut 8 % de ce qu'une campagne rapporte au plus (120 à 150 % voulus). **(2) non tenu** : tout s'achète, avec plus de 13 000 ¢ de reste. **(3) non tenu** : les quêtes principales du palier 1 paient 80 ¢ pour un objet de 150, le palier 2 150 pour 200, le palier 3 400 pour 500 (les paliers suivants suffisent). **(4) tenu** : le `hoarder` est bloqué dans 216 parties sur 216 (par `stealth_module`, la quête M02) ; sur les cinq objets, quatre sont demandés par une quête, seul `ghost_protocol` (80 ¢) est un choix.
- **Constat pour le propriétaire** : la spec supposait un étal de 15 à 18 k¢ (programmes et améliorations à acheter), alors que dans le jeu construit les programmes viennent du palier et non de l'étal : l'argent n'a presque aucun puits. Soit l'étal grossit (programmes, améliorations, services plus chers), soit les revenus baissent, soit la spec change ; c'est un réglage de conception, pas du code du banc.

**Les programmes à l'étal (R6.8, [`DECISIONS.md`](../design/DECISIONS.md) R-37).** Pour grossir l'étal (constat de R6.7), le propriétaire a choisi de **vendre les programmes** : un objet de `catalog.toml` peut porter `program = "…"` ; un programme que **un objet** accorde est « vendu », et le deck d'une intrusion est le deck du palier du site **sans** les programmes vendus dont le joueur ne possède pas l'objet (`Content::deck`, `Stock::Owned`). Les autres programmes (`brute`, `backdoor`, `cloak`) sont le deck de base. Les outils (`solve`, `economy`, les preuves T2 et T4, `sites::measure`) gardent le deck complet du palier (`Stock::Everything`) : leurs résultats ne changent pas. Huit objets (`prog_exploit`, `prog_decrypt`, `prog_spoof` au palier 2, `prog_quantum` et `prog_social` au 3, `prog_virus` et `prog_ghost` au 4, `prog_overclock` au 5) à 80-900 ¢ ; l'optimiste (`brain`) achète, avant d'entrer sur un site, les programmes du deck de ce site qu'il peut payer, le moins cher d'abord ; un `hoarder` n'en achète aucun et est **bloqué** par l'objet d'une quête ou par un deck trop mince pour le site.

- **Mesure** (216 plans, graine 1) : l'`optimist` termine 864 parties sur 864 aux quatre difficultés (T1 tient avec des programmes achetés), le `farmer` 216 sur 216 en Histoire, Normal et Expert avec les mêmes crédits et la même réputation (T8 tient), le `hoarder` est **bloqué** dans 864 parties sur 864 (Histoire et Normal par `stealth_module`, Expert et Hardcore par le deck du site : sans programme acheté, `nexus-mainframe` n'a pas de plan) ; `prudent`, `reckless` et `story` (8 plans représentatifs) se comportent comme en R6.4 et R6.5. L'étal passe de 5 objets et 1 730 ¢ à **13 objets et 4 460 ¢** : T7 reste non tenu (l'étal vaut 23 % de ce qu'une campagne rapporte au plus, 120 à 150 % voulus), les prix suivant pour l'instant la courbe des revenus.
- **Effet de jeu** : sans les programmes les moins chers (`exploit`), l'optimiste joue le début avec `brute` : plus bruyant, la Notoriété du début passe de 29 (Discret) à 31 (Surveillé) dans le script des trois premières quêtes.

**Les programmes et T9 (R6.9, [`DECISIONS.md`](../design/DECISIONS.md) R-38).** `neon-sim programs [--site ID] [--expansions N] [--out FICHIER]` mesure, à chaque site et en Normal, avec l'objectif seul et la recherche exacte du solveur, la marge avec **tout le deck du palier** puis sans chacun de ses programmes (`kind = programs`). Pour chaque programme : les sites qu'on ne peut plus gagner sans lui (`needed_at`), ceux où la marge baisse (`lowers`, avec de combien), ceux où il ne change rien (`idle_at`), le gain total, et s'il est **dominé** (aucun site ne le demande et aucun n'y perd de marge). **T9 en objectif (O)** : chaque programme baisse la marge d'un site quand on l'ôte, ou un site en dépend.

- **Mesure du contenu livré** (16 sites, Normal ; 169 mesures, toutes prouvées, aucune « inconnue » ; 3 min 30 en version de publication) : **aucun programme n'est indispensable** (le deck de base `brute`, `backdoor`, `cloak` gagne tous les sites, ce qui est la preuve T2), et **T9 n'était pas tenu (avant R6.13) : `exploit` (100 ¢) et `social` (200 ¢) ne font gagner de marge sur aucun site**. Valeur des autres (marge ajoutée, tous sites confondus) : `spoof` (80 ¢) +69 sur 15 sites, `cloak` +63 sur 14 (deck de base), `ghost` (500 ¢) +26 sur 10, `overclock` (900 ¢) +23 sur 8, `backdoor` +16 sur 7 (base), `decrypt` (100 ¢) +16 sur 11, `brute` +14 sur 9 (base), `quantum` (250 ¢) +10 sur 5, `virus` (600 ¢) +5 sur 2 (`gov-database` +1, `radio-veille` +4). Les gains sont petits (1 à 12 points d'une marge de 25 à 70).
- **Décision du propriétaire (R6.13, [`DECISIONS.md`](../design/DECISIONS.md) R-39)** : les programmes inutiles sont retirés. `exploit` et `social` ne sont plus dans aucun deck ni à l'étal (plus d'objet `prog_exploit` ni `prog_social`), et les nœuds `intel`, que seul `social` lisait, deviennent des `extra`. Le moteur garde ces deux programmes dans ses règles (ses tests s'en servent), le jeu ne les donne plus. **T9 est tenu** : chacun des 9 programmes qui restent fait baisser la marge d'un site ; `virus` (600 ¢) ne sert que sur deux sites et reste.
- La seconde clause de T9 (« aucun programme ne bat un autre sur tous les sites ») comparée par gains de marge site par site range presque tout sous `spoof` : elle ne dit rien d'utile sur ce contenu et n'est pas mesurée.

## 8. Procédure de réglage (R6)

1. Lancer la campagne complète pour tous les bots et difficultés ; lire le rapport.
2. Corriger **une famille de nombres à la fois** (prix, récompenses, force d'ICE, cycles, bruit), jamais le code.
3. Relancer ; vérifier que les barrières P restent vertes et que les objectifs O se rapprochent.
4. Quand un objectif O est atteint et validé, le promouvoir en P dans `targets.toml`.
5. Consigner dans `DECISIONS.md` chaque valeur arrêtée avec le rapport qui la justifie.

## 9. Limites et décisions

- Les bots ne reproduisent pas un humain : ils bornent le jeu (plancher, plafond, robustesse), ils ne mesurent pas l'ennui ni la lisibilité. R7 y ajoute un essai humain.
- La durée dépend de la cadence supposée (§ 6) et du volume de texte lu, que le moteur ne connaît pas.
- Le *bot solveur* dépend de la borne du solveur et de son repli ([`solver.md`](solver.md)) ; les seuils T2-T4 et T13 sont recalés sur les mesures du spike, mais le contenu final reste à écrire : ce sont des **cibles**, pas encore des mesures.
- **S-8** (validée le 8 octobre 2026, [`DECISIONS.md`](../design/DECISIONS.md) § 6) : le principe des seuils en données, les barrières P/O du § 5 comme point de départ, et la cadence de 8 commandes par minute.
