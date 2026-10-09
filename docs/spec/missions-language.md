# Référence du langage de missions

Cette référence décrit le langage de missions tel que `neon-engine` l'exécute (module `neon_engine::content`, lot R2.1). Elle est née du spike de la phase P1 (`spikes/missions`, supprimé une fois porté) qui répondait au point 12 du [cross-check](../design/CROSS-CHECK.md) : le prototype n'avait que 3 conditions ; la bible demande une vingtaine de types d'objectifs, une dizaine de conditions, des effets, 7 états de contact et des drapeaux typés. Les décisions qu'elle applique sont dans [`missions.md`](missions.md) ; les paragraphes du spike (« le spike prend… ») décrivent les choix faits à l'époque et inscrits dans les données.

**Résultat.** Un langage déclaratif **petit** (14 objectifs, 9 conditions dont 3 combinateurs, 9 effets, 4 types de drapeaux, 5 statuts de quête, 7 états de contact, 16 faits) exprime **les quêtes M01 à M14 de la bible en entier**, plus S06, S07, S08, S10, S11, S12, S13, S14 et S16, les trois décisions D1-D3 et la micro-décision `echo_trust`, les 5 fins, le montage d'épilogue et des sujets de dialogue. Un « joueur optimiste » rejoue tout le contenu pour **216 plans** (D1 × D2 × D3 × `echo_trust` × objectifs optionnels) et finit toujours M14 : pas d'impasse. Ce spike a trouvé **une dizaine d'incohérences de la bible** (§ 8) que seule l'exécution fait apparaître (trois sont de vraies impasses : M01 #5, M03 #3, et la règle A contre la colonne « Palier »).

Rien dans `docs/design/` n'est réécrit (R-0) ; les écarts sont listés au § 8.

## 1. Où est quoi

```
data/world/
  catalog.toml      sites (+ fichiers), objets, contacts, « readables » (fragments, courriers, scènes, documents), commandes
  flags.toml        drapeaux typés (bool, enum, compteur, bitset)
  rewards.toml      tailles S/M/L/XL, R(P) par palier, constantes de confiance, crédits de départ ;
                    constantes du hub (R2.2b) : services de `laylow`, Notoriété nominale d'une intrusion
                    automatique par niveau de site, budgets d'indices par difficulté
  quests.toml       [[quest]] M01-M14 + 9 contrats
  decisions.toml    [[decision]] D1-D3 et echo, [[ending]], [[epilogue]]
  topics.toml       [[topic]] (sujets de dialogue)
  texts.toml        clés de texte déclarées (dérivées, voir § 6.8)
  unlocks.toml      ouverture des commandes du jeu : `command`, `when` (une condition), `reason` (clé `unlock.<nom>`),
                    validée contre la table de commandes (R2.2b, docs/spec/commands.md § 6)
crates/neon-engine/src/content/
  ids.rs            ids textuels typés (un type par espace de noms, validés à la création)
  money.rs          Credits et Reputation, saturants (plafonds 10^9 et ±10^6, invariant I11)
  schema.rs         ce que le TOML peut dire (serde, deny_unknown_fields)
  loader.rs         chargeur, index, erreurs « fichier:ligne » ; données embarquées par build.rs
  validate.rs       toutes les règles de validation
  state.rs          État, Fact (entrée du bus), registre idempotent
  eval.rs           conditions, mesure des objectifs, journal et progression (fonctions pures)
  engine.rs         refresh (point fixe), effets, Outcome (sortie)
  texts.rs          dérivation des clés, budgets de largeur (unicode-width)
  persist.rs        sérialisation de l'état (clés textuelles) et State::validate
  view.rs           lectures pour les commandes du hub (quêtes par statut, contacts, fin)
crates/neon-engine/tests/content/
  optimist.rs       le joueur optimiste (outil de test, hors moteur)
  language, validation, regressions, walk, props, persist, shipped, view   les tests
```

Commandes : `cargo test -p neon-engine` ; pour régénérer `data/world/texts.toml` après avoir ajouté une quête : `NEON_REGEN_WORLD_TEXTS=1 cargo test -p neon-engine --test content regenerate_the_declared_text_keys -- --ignored`.

## 2. Principes

1. **Un seul langage de conditions, un seul langage d'effets**, utilisés par les quêtes, les objectifs (`when`), les décisions (`requires`), les sujets de dialogue, les fins et l'épilogue. Aucun second langage.
2. **Le langage ne contient aucun texte** : des ids et des clés dérivées (§ 6.8).
3. **Le moteur ne rappelle personne** : `apply(fait)` ne fait qu'inscrire le fait dans un *registre* (ensembles, maxima, front de Pareto) ; `refresh` lit le registre, déplace les statuts, paie les récompenses et renvoie une liste d'`Outcome`. La chaîne « une quête finie en débloque une autre » est un **point fixe itératif borné**, pas une récursion ni une re-livraison par le bus.
4. **Statut posé avant paiement**, et chaque paiement passe par une clé de réclamation unique (`quest-m05`, `breach-nexus-mainframe`, `decision-d1`...) : deux gardes indépendantes (I1, I8).
5. **Déterminisme** : `BTreeMap`/`BTreeSet`, tableaux `[[...]]` dans l'ordre du fichier, aucun temps réel (le temps est un compteur de tours, `Turn`).

## 3. Référence du langage

### 3.1 Identifiants et ressources

Ids textuels `[a-z0-9][a-z0-9_-]{0,47}`, un type par espace de noms : `QuestId`, `SiteId`, `FileId`, `ContactId`, `ItemId`, `ReadableId`, `FlagId`, `DecisionId`, `ChoiceId`, `TopicId`, `CommandId`, `EndingId`, `LineId`. Les noms d'origine (« Stealth Module v2.0 », `DOC_LEDGER`, `NEXUS_DATA_BREACH`, `neural_maps.bin`) sont gardés en champs `legacy_name` / `legacy_code` / `code` / `file_name`, pour la traçabilité seulement (la bible 6.3 écrit `quantum_key`, `nexus-mainframe`, `angel`, `cs03` en minuscules : on la suit).

Un **readable** est tout ce qui s'ouvre : `fragment` (F01-F24), `mail`, `scene` (cinématique ou écran de fin), `document` (chiffré). Il a une *source* obligatoire (`start`, un fichier ou un document qui le `yields`, ou un effet `unlock`) : un fragment sans source est une erreur (test T9).

### 3.2 Types de drapeaux (`flags.toml`)

| Type | Déclaration | Valeur | Exemple |
|---|---|---|---|
| `bool` | — | `true`/`false` (défaut `false`) | `lull_done`, `broker_retainer` |
| `enum` | `values = [...]` (le premier est le défaut) | une des valeurs | `phoenix_state`, `d1`, `ending` |
| `counter` | `max = n` | 0..=n | `echo_trust` (max 2), `chapter` (max 6) |
| `bitset` | `values = [bits]` | ensemble de bits nommés | `adv` (A1-A6) |

Toute condition ou tout effet qui emploie une valeur non déclarée est une erreur de chargement. `set_flag` sur un bitset **ajoute** le bit nommé.

### 3.3 Statuts de quête

| Statut | Sens | Entrée | Sortie |
|---|---|---|---|
| `UNAVAILABLE` | prérequis, palier, donneur ou `available_if` non satisfaits | état initial | → `AVAILABLE` |
| `AVAILABLE` | **proposée** par son donneur, en attente d'acceptation | tout est réuni | → `ACTIVE` (acceptation), → `UNAVAILABLE` (la condition devient fausse avant l'acceptation) |
| `ACTIVE` | en cours ; les objectifs d'événement comptent depuis `opened_at` | `main` : aussitôt ; `side`/`branch` : fait `QuestAccepted` | → `COMPLETED`, → `FAILED` |
| `COMPLETED` | terminée (terminal) | tous les objectifs requis tenus | — |
| `FAILED` | perdue (terminal) | `fail_if` vrai ou objectif `heat_peak_below` perdu, **quêtes `failable` seulement** | — |

`kind = "main"` s'ouvre toute seule ; `side` et `branch` (S07, ouverte par D2) sont *proposées* (courrier ou dialogue : c'est le statut `AVAILABLE` que le moteur actuel réservait) et le joueur les *accepte*. Une quête `ACTIVE` n'est plus soumise à `available_if`.

### 3.4 États de contact

`available`, `busy`, `offline`, `compromised`, `hostile`, `silenced`, `dead` (bible 5.6). `offline` sert aussi d'état « pas encore débloqué » : tous les contacts sauf ECHO-7 commencent `offline` et une quête les débloque par `set_contact_state`. Règles : on **parle** à `available`, `compromised`, `silenced` ; un contact **donne** des quêtes en `available` ou `compromised`. Les finesses narratives (`phoenix_state` : ally/compromised/free/lost/turned ; `angel_state`) sont des drapeaux enum, pas des états de contact : l'état de contact reste le résumé à 7 valeurs.

### 3.5 Faits (entrée, ce que le bus émet)

`SiteCompromised{site,at}`, `FileExtracted{site,file}`, `SiteMarked{site,mark}`, `Read{id}`, `Decrypted{id}`, `ItemBought{item}`, `Talked{contact,at}`, `CommandUsed{command}`, `LinkChanged{contact,level}`, `Paid{amount,at}`, `HeatChanged{heat,at}`, `DecisionMade{decision,choice}`, `TopicChosen{contact,topic}`, `QuestAccepted{quest}`, `QuestRestarted{quest,at}`, `Tick{at}`. Les faits répétables portent le tour `at` : deux conversations sont deux valeurs distinctes, et « depuis l'ouverture » se lit `at > opened_at`. `ContactMet` de la bible = `Talked` (voir § 3.6) ; `WorldPhaseChanged`, `HubChanged`, `ContactStateChanged` sont des changements de drapeau ou d'état renvoyés en `Outcome`.

### 3.6 Objectifs (14 variantes pour les 22 noms de la bible)

Un objectif est un tableau `[[quest.objective]]` : `kind` + champs utiles, plus les indicateurs `id`, `optional`, `secret`, `when`. Une faute de forme (champ manquant, champ inutile, `count` et `site` ensemble) est une erreur à la ligne de l'objectif. **Mode** : *événement* = ne compte que ce qui arrive **après** l'ouverture de la quête ; *stock* = lit l'état, quand qu'il ait été atteint (§ 6.2).

| `kind` | Champs | Mode | Remplace (bible 5.1) |
|---|---|---|---|
| `talk` | `contact`, `count` (déf. 1) | événement | `MEET`, `TALK` (`MEET` = `count 1`) |
| `buy` | `item` | stock | `BUY` (l'alternative = `any_of`) |
| `compromise` | `site` **ou** `count` (n sites différents) | événement | `COMPROMISE`, `COMPROMISE_ANY` |
| `extract` | `site` + `file`, ou `site` + `all`, ou `[site]` + `count` | stock | `EXTRACT` (fichier, tous, n) |
| `open` | `readable` | stock | `READ`, `DECRYPT`, `DECRYPT_QUANTUM`, `CHOICE`... (voir note) |
| `link` | `contact`, `level` 1-3 | stock | `LINK` |
| `reputation` | `min` | stock | `REPUTATION` |
| `use` | `command` **ou** `group` | stock | `USE`, `USE_COVER` |
| `site_state` | `site`, `mark` (`backdoor`/`virus`/`analyzed`) | stock | `BACKDOOR`, `UPLOAD_VIRUS`, `ANALYZE` |
| `pay` | `amount` (S/M/L/XL, résolu au palier de la quête) | événement | `PAY` |
| `choice` | `decision` | stock | `CHOICE` |
| `heat_end_below` | `below` | **condition** (au moment de conclure) | `HEAT_END_BELOW` |
| `heat_peak_below` | `below` | **condition**, peut être *perdu* | `HEAT_PEAK_BELOW` |
| `any_of` | `of` (2 à 4 alternatives, sans imbrication) | selon les enfants | `UNE_DE` |

Note : `DECRYPT_QUANTUM` n'est plus un type : c'est `open` sur un document dont le catalogue dit `requires_item = "quantum_chip"` (`doc_vault`). `REACH_TIER` n'existe plus (inatteignable, § 8). `CHOICE` reste distinct de `open` (la décision a ses options).

**Indicateurs.** `optional` (bonus : ne bloque pas) ; `secret` (le journal montre `???` tant qu'il n'est pas accompli, voir `eval::journal`) ; `condition` (**dérivé du type** : `heat_end_below` et `heat_peak_below` ne sont jamais enregistrés, ils doivent être vrais à l'instant de conclure) ; plus `when`, une condition (§ 3.7) qui rend l'objectif *applicable* ou non (utilisé pour le demi-tarif de D1 : `pay L` si pas de `broker_retainer`, `pay M` sinon) ; plus `id`, pour que `Cond::Objective` puisse tester « cet objectif, ou cette alternative, a été accompli ».

### 3.7 Conditions (9)

`all{of}`, `any{of}`, `not{of}` ; feuilles : `flag{name, is | at_least | has}`, `quest{id, is}`, `contact{id, is=[états]}`, `trust{contact, at_least}`, `opened{id}`, `objective{quest, id}`. Le chapitre est un compteur (`flag chapter at_least n`). Profondeur maximale 6 (validée). Une condition est un arbre fini de lectures : pas de boucle, pas d'écriture.

### 3.8 Effets (9) et blocs

Un **bloc** est `{ when = <condition>, then = [effets] }` (`when` facultatif) ; quêtes (`on_open`, `on_complete`, `on_fail`), choix de décision (`then`) et sujets (`then`) sont des listes de blocs, évalués dans l'ordre (un bloc voit les écritures des précédents).

| Effet | Rôle |
|---|---|
| `set_flag{flag,value}` | pose un drapeau (ajoute le bit d'un bitset) |
| `set_contact_state{contact,state}` | débloque / ferme un contact |
| `trust{contact,delta}` | confiance gagnée ou perdue par un choix |
| `grant{credits?,reputation?,penalty?}` | paiement symbolique (S/M/L/XL × R(P)) |
| `grant_tier{tier}` | monte le palier (jamais ne le baisse) |
| `unlock{readable}` | livre un courrier, joue une scène, révèle un fragment |
| `heat_force{value}` | chaleur forcée (trahison B1) → `Outcome::HeatForced` |
| `heat_floor{delta,until}` | plancher de chaleur jusqu'à l'ouverture d'une quête (B2, D2) |
| `settle_ending` | écrit dans le drapeau `ending` la première `[[ending]]` dont la condition tient |

Le champ `reward = { credits = "M", reputation = "S" }` d'une quête est un `grant` payé à la conclusion, avec +10 de confiance au donneur (`rules.quest_completed`).

### 3.9 Décisions, fins, épilogue, sujets

* `[[decision]]` : `flag` (enum dont les valeurs sont les choix), `quest` (qui porte l'objectif `choice` et le palier), `[[decision.choice]]` avec `requires` (condition d'offre) et `then`. Le drapeau est posé automatiquement. `DecisionMade` n'est qu'inscrit ; l'effet part dans `refresh`, **avant** les quêtes du même passage, donc `settle_ending` voit `d3`.
* `[[ending]]` : `when` + `paragraphs` ; **première correspondance** dans l'ordre du fichier (E1a avant E1b).
* `[[epilogue]]` : une ligne = un contact + `when`. Test : exactement une ligne par contact, pour les 216 plans.
* `[[topic]]` : `contact`, `when`, `then`. Disponible si le contact est joignable et le `when` tient.

### 3.10 Récompenses et déblocages

Aucun montant dans les quêtes : `rewards.toml` donne `credits_percent` et `reputation` de S/M/L/XL et `R(P)` par palier (80, 150, 200, 500, 800, 800). Le champ `tier` d'une quête est son palier de référence `P` (récompenses, `pay`) ; son **verrou** est `unlock = { tier = n }` (ou `{ quest = "id" }`, aussi pour les sites, objets et readables). Les paliers sont accordés par la ligne principale (`grant_tier`) : M01 → 2, M02 → 3, M04 → 4, M06 → 5, M10 → 6 (colonne « Palier » de la bible).

## 4. Chargement et validation

`Content::from_sources(&Sources)` prend des chaînes (le moteur ne fait pas d'E/S), renvoie `Result<Content, LoadError>` ; `LoadError` porte des `Diagnostic { file, line, message }` affichés `data/world/quests.toml:418: quest `m09`, objective 2: unknown site `freeport-0x``. Un `toml::Spanned` donne la ligne de chaque entrée (`[[quest]]`, `[[decision]]`...) et de chaque objectif. Une erreur de syntaxe ou de schéma arrête le fichier fautif ; la validation sémantique, elle, **rapporte toutes les erreurs** triées par fichier et ligne.

Règles vérifiées (13 tests négatifs dans `tests/content/validation.rs`, 3 de plus dans `tests/content/regressions.rs`) : ids uniques et bien formés ; toute référence existe (quêtes, contacts, sites, fichiers d'un site, objets, readables, drapeaux, décisions, commandes) ; prérequis **et `unlock.quest`** acycliques (Kahn, itératif), écrits avant la quête, jamais auto-référents ; relais de sites acycliques, palier monotone le long des relais ; types de drapeaux cohérents, valeurs d'enum/bit déclarées, bornes de compteur ; tailles et paliers de récompense résolus ; objets requis ; au plus **8 objectifs** par quête et 4 alternatives ; profondeur de condition ≤ 6 ; `failable` cohérent (jamais une quête principale) ; chaque décision a un objectif `choice` dans sa quête ; chaque readable a une source *atteignable* (point fixe depuis `start`, les fichiers et les effets `unlock` : deux documents qui se livrent l'un l'autre n'en ont pas) ; les clés de texte déclarées sont exactement les clés dérivées, et deux ids qui donnent la même clé après `-` → `_` (`foo-bar`, `foo_bar`) sont refusés ; **deux écritures du même drapeau non ordonnées** (ni par prérequis, ni choix exclusifs d'une même décision, ni même valeur) sont refusées, ce qui garantit que le résultat ne dépend pas de l'ordre des événements.

**N = 8 objectifs.** La bible a M01 à 7 et M12 à 6 ; la limite du C (5) est donc relevée. 8 tient en une ligne chacun dans la zone du journal de la disposition 64×20 (≈ 12 lignes utiles) et laisse un objectif de marge à M01 ; au-delà on découpe la quête ou on groupe avec `any_of`. Le texte de chaque objectif est limité à 64 colonnes.

## 5. Exemple : M04 et M05

```toml
[[quest]]
id = "m04"                       # « Suivre l'argent » : Data Miner suit les fonds
code = "FOLLOW_THE_MONEY"
kind = "main"                    # s'ouvre seule dès que M03 est finie
chapter = 2
giver = "miner"                  # doit être joignable (débloqué par M03)
tier = 3                         # palier de référence : M = 100 % de R(3) = 200 ¢, +25 de réputation
unlock = { tier = 3 }
prereq = ["m03"]
reward = { credits = "M", reputation = "M" }
[[quest.objective]]              # 1. tous les fichiers de corp-server-01 (stock : le butin est unique)
kind = "extract"
site = "corp-server-01"
all = true
[[quest.objective]]              # 2. le grand livre
kind = "extract"
site = "underground-market"
file = "black_ledger"            # ce fichier livre doc_ledger (chiffré)
[[quest.objective]]              # 3. le décrypter (doc_ledger livre F04)
kind = "open"
readable = "doc_ledger"
[[quest.objective]]              # 4. condition : chaleur < 50 au moment de conclure
kind = "heat_end_below"
below = 50
[[quest.on_complete]]
then = [
    { kind = "grant_tier", tier = 4 },
    { kind = "unlock", readable = "f06" },
]

[[quest]]
id = "m05"                       # « L'œil du Cyclone »
code = "NEXUS_DATA_BREACH"
kind = "main"
chapter = 3
giver = "phoenix"
tier = 4
unlock = { tier = 4 }
prereq = ["m04"]
reward = { credits = "L", reputation = "XL" }
[[quest.on_open]]
then = [{ kind = "set_flag", flag = "chapter", value = 3 }]
[[quest.objective]]              # 1. UNE_DE : n'importe lequel des trois achats
kind = "any_of"
of = [
    { kind = "buy", item = "quantum_key" },
    { kind = "buy", item = "neural_assistant" },
    { kind = "buy", item = "quantum_chip" },
]
[[quest.objective]]              # 2. événement : compter seulement APRÈS l'ouverture de M05
kind = "compromise"
site = "nexus-mainframe"
[[quest.objective]]              # 3. stock : un fichier de nexus-mainframe
kind = "extract"
site = "nexus-mainframe"
[[quest.on_complete]]
then = [
    { kind = "unlock", readable = "f08" }, { kind = "unlock", readable = "f09" },
    { kind = "unlock", readable = "f10" }, { kind = "unlock", readable = "cs03" },
    { kind = "unlock", readable = "mail-aura" }, { kind = "unlock", readable = "mail-angel" },
    { kind = "set_contact_state", contact = "aura", state = "available" },
    { kind = "set_contact_state", contact = "angel", state = "available" },
    { kind = "set_contact_state", contact = "broker", state = "busy" },
]
```

**Déroulé** (`tests/content/language.rs`, `m04_*`, `m05_*`) : M03 se termine au `Decrypted(doc_phase2)` ; dans le même `refresh`, ses effets débloquent Data Miner et Phoenix, M04 passe `AVAILABLE` puis `ACTIVE` (`opened_at` = tour courant) mais **ne se termine pas** dans ce passage, même si le butin était déjà pris : un tour plus tard (`Tick`), les objectifs de stock tiennent et M04 est `COMPLETED` (palier 4, 200 ¢, +25). Si la chaleur est à 70, M04 reste `ACTIVE` (aucun échec) et se conclut dès qu'elle retombe sous 50. M05 s'ouvre (palier 4, chapitre 3) ; l'achat de `quantum_key` suffit pour l'objectif 1 ; l'objectif 2 **ignore** la brèche de nexus-mainframe faite pendant M03 et attend une nouvelle brèche ; à la fin : 1 000 ¢ (L à P4 = 2 × 500) et +100 de réputation (XL), versés une fois (`Output::Reward { key: "quest-m05" }` ; un effet `grant` d'une source porte sa propre clé `quest-m05:<bloc>.<effet>`, donc toutes les clés de `Reward` sont uniques), AURA, Neon Angel débloqués, le Courtier `busy`.

## 6. Réponses à la checklist

### 6.1 Types réellement nécessaires

* **Objectifs** : 22 noms dans la bible → **14 variantes** (§ 3.6), après fusion : `MEET` = `TALK` ; `COMPROMISE_ANY` = `COMPROMISE` sans site ; `USE_COVER` = `USE` par groupe ; `BACKDOOR`/`UPLOAD_VIRUS`/`ANALYZE` = `site_state` ; `DECRYPT`/`DECRYPT_QUANTUM`/`READ` = `open`. Abandonné : `REACH_TIER` (inatteignable). Les 14 sont tous employés par les données livrées (`talk` 9 fois, `compromise` 14, `extract` 15, `open` 9, `site_state` 8, `heat_end_below` 7, `buy` 6, `link` 5, `use` 5, `pay` 5, `choice` 4, `reputation` 3, `any_of` 2, `heat_peak_below` 1).
* **Conditions** : la bible en demande une dizaine ; **9** suffisent (6 feuilles + `all`/`any`/`not`). Les conditions `tier`, `reputation`, `item` et `extracted`, écrites d'abord, n'ont servi à rien : supprimées. Le chapitre est un compteur de drapeau, « fragment lu » est `opened`, l'état d'une quête est `quest`.
* **Effets** : **9** (le cahier en réclamait ≥ 8) ; employés 27, 18, 9, 2, 5, 32, 2, 3 et 1 fois. Les événements `WorldPhaseChanged`, `HubChanged`, `NodeReset`, `CutsceneSeen` de la bible 5.6 disparaissent : ce sont un drapeau, un drapeau, la règle « événement depuis l'ouverture », et un `unlock` de scène.
* **Drapeaux** : 4 types (bool, enum, compteur, bitset), 16 drapeaux. **Statuts** : 5. **États de contact** : 7. **Faits** : 16 (dont `Tick`).

### 6.2 Objectif déjà vrai à l'ouverture (audit 2.7)

Règle **R-OPEN**, fixée par le *type* d'objectif (jamais par un réglage par quête) :

1. **Objectifs d'événement** (`compromise`, `talk`, `pay`, et `heat_peak_below`) : ne comptent que les faits d'un tour `at > opened_at`. Une brèche de nexus-mainframe faite avant M05 ne compte pas : c'est ce qui empêche l'enchaînement instantané signalé par l'audit.
2. **Objectifs de stock** (`buy`, `extract`, `open`, `link`, `reputation`, `use`, `site_state`, `choice`) : lisent l'état, **rétroactivement**. Raison : ce sont des choses uniques (butin, achat, document déchiffré, décision) ; exiger de les refaire après l'ouverture créerait une impasse (M04 demande tout le butin de corp-server-01, déjà pris en M02 si le joueur a été consciencieux).
3. **Aucune quête ne se conclut dans le `refresh` qui l'ouvre** : `clock > opened_at` est exigé. Une quête 100 % stock (M04) reste donc visible au moins un tour : son annonce, sa scène d'ouverture et son `on_open` ne sont pas escamotés. Coût : un `Tick` du moteur (un tour de jeu) avant de conclure ; le joueur optimiste en émet un quand il n'a plus rien à faire. Le joueur optimiste réserve aussi les coûts qu'il met en file dans un tour (achats et paiements) : il ne dépense jamais plus que son solde.

### 6.3 AVAILABLE contre ACTIVE

`AVAILABLE` = la quête est **proposée** (un `QuestAccepted` reçu avant l'offre est ignoré, et une offre retirée oublie l'acceptation) (courrier, sujet « mission » du donneur) ; elle n'a pas de `opened_at`, aucun objectif ne compte, et elle peut redevenir `UNAVAILABLE` si son `available_if` devient faux (S08 si R4Z0R devient hostile). `ACTIVE` commence au fait `QuestAccepted` (contrats) ou tout de suite (quêtes principales) et fixe `opened_at`. `tests/language.rs::contracts_are_offered_then_accepted`.

### 6.4 FAILED (S12 seulement)

`failable = true` est obligatoire pour `fail_if`/`on_fail` et **interdit aux quêtes principales** (validé : un échec principal serait une impasse). S12 échoue si l'objectif `heat_peak_below 60` est perdu (pic de chaleur ≥ 60 depuis l'ouverture) ou si M13 s'ouvre (`fail_if = quest m13 active`) ; `on_fail` rend Mizuki `compromised`, une fois (`quest-s12-failed`). L'achèvement est évalué avant l'échec : finir S12 au tour où M13 s'ouvre compte comme réussi. Une quête **non** `failable` dont le pic est perdu reste `ACTIVE` (bloquée) jusqu'au fait `QuestRestarted`, qui rouvre la quête **au tour du redémarrage** (pas à la fin du lot : un fait du tour suivant, dans le même lot, compte) (test `a_lost_peak_on_a_quest_that_cannot_fail_waits_for_a_restart`).

### 6.5 Sujets de dialogue

`[[topic]]` = `{ contact, when?, then? }` avec exactement les `Cond` et `Effect` des quêtes (`when = any[quest m04 active|completed]`, `then = [trust +5]`). Disponible si le contact est joignable (états `available`/`compromised`/`silenced`, donc **aucun** sujet pour un contact `offline`, `busy`, `hostile` ou `dead`) ; l'effet part une fois (clé `topic-<contact>-<id>`). Les 7 sujets livrés couvrent « chapitre ≥ n » (compteur), état de quête, drapeau, confiance, état de contact. `TopicChosen` est invalidé et retiré du registre si le sujet n'était pas offert.

### 6.6 Épilogue sans second langage

Les fins sont des `[[ending]]` (`when` = `Cond`, première correspondance) ; M13 les « fige » par `settle_ending` ; le montage est une liste de `[[epilogue]]` (contact + `when`). La matrice de la bible (4.4) devient : `e1a` si `d3 = liberate` ∧ `adv ∋ echo_log` ∧ `echo_trust ≥ 1`, `e1b` si `d3 = liberate`, puis `e2`, `e3`, `e4` selon `d3`. `tests/content/walk.rs` joue les 216 combinaisons et vérifie : **une et une seule fin** par combinaison (ni `none`, ni deux), les 5 fins sont atteintes, et **exactement une ligne d'épilogue par contact**. Un test écrit la fonction attendue indépendamment des données.

### 6.7 Piège de Turing

Aucun, par construction : (1) pas de boucle ni de variable : le seul état modifiable est l'ensemble des drapeaux **déclarés** ; (2) les conditions sont des arbres finis de profondeur ≤ 6, sans appel ; (3) les effets ne déclenchent pas d'effets : ils écrivent et émettent des `Outcome` ; (4) chaque effet part **au plus une fois** (clé de réclamation) donc le point fixe de `refresh` est borné (`4 × quêtes + décisions + sujets + sites + 16` passes, vérifié par propriété : `BudgetExceeded` n'arrive jamais) ; (5) les écritures concurrentes d'un drapeau sont refusées au chargement. Ce qui *ne* peut pas s'écrire : une quête qui compte jusqu'à n sans objectif dédié, un effet qui dépend d'un compteur arbitraire, une condition sur un nombre d'événements passés autre que `talk`/`pay`/`compromise` « depuis l'ouverture ».

### 6.8 Dérivation des clés de texte et budgets de largeur

Règle (`texts::required_keys`), `-` des ids devenant `_` : `quest.<id>.{title,title_short,desc,lore,loc,debrief}`, `quest.<id>.obj.N` et `quest.<id>.hint.N.1` (+ `.2` pour les quêtes principales, test T13) pour chaque objectif de premier niveau ; `contact.<id>.{name,tagline}`, `contact.<c>.topic.<t>.{q,a}` ; `frag.<id>.{title,body}`, `mail.<id>.{subject,body}`, `cutscene.<id>.pNN` (`paragraphs` du catalogue), `doc.<id>.title` ; `decision.<id>.prompt`, `decision.<d>.choice.<c>.label` ; `ending.<id>.{title,pNN}` ; `epilogue.<ligne>` ; `node.<site>.desc`, `file.<site>.<fichier>.desc` ; `item.<id>.{name,desc}` ; `command.<id>.help`. Les plus de 600 clés de `texts.toml` sont **égales** aux clés dérivées (test) : en ajouter ou en oublier une est une erreur de chargement, avec la clé en cause. Budgets : `texts::budget(clé)` donne la largeur maximale d'après la bible 6.2 (titre 28, `title_short` 18, `desc` 110, objectif 64, indice 160, `debrief` 200, `tagline` 40, sujet 48/340, courrier 48/600, cinématique 280, fragment 40/520, épilogue 300, nœud 60, fichier 40), et `texts::check_budgets(content, texte_de_la_clé, mesure)` rapporte les dépassements pour un catalogue de langue. La mesure est un paramètre ; la mesure réelle est `texts::display_width` (`unicode-width`, largeur en colonnes de terminal), et `texts::check_catalog_budgets` / `texts::missing_keys` branchent les budgets et la parité sur un vrai `Catalog`. Tests : `budgets_are_hooks_the_catalogs_plug_into`, `the_real_catalogs_stay_within_the_budgets_of_the_keys_they_define`.

### 6.9 Taille du portage (lot R2.1)

Lignes de `crates/neon-engine/src/content/`, documentation et tests unitaires compris (le spike en comptait ≈ 3 100 hors documentation) :

| Module | Lignes | Rôle |
|---|---:|---|
| `schema.rs` | ≈ 1 050 | types serde, conversion des objectifs, documentation de chaque champ |
| `loader.rs` | ≈ 450 | chargeur, index, erreurs, données embarquées |
| `validate.rs` | ≈ 1 100 | validation (la plus grosse part) |
| `state.rs` + `eval.rs` + `engine.rs` | ≈ 1 300 | l'évaluateur et le moteur de `refresh` |
| `ids.rs`, `money.rs`, `texts.rs`, `view.rs` | ≈ 650 | ids typés, montants saturants, clés de texte, lectures du hub |
| `persist.rs` | ≈ 565 | sérialisation de l'état et `State::validate` |
| `tests/content/optimist.rs` (outil de test) | ≈ 510 | le joueur optimiste |

Les données : ≈ 750 lignes de quêtes, ≈ 440 de catalogue, ≈ 250 de décisions, pour 23 quêtes.

## 7. Le reste de la bible : M07, M09/D1, S07, S12, M13, M14

La consigne demandait de le montrer « sur le papier » ; tout ce qui suit est en fait **dans les données livrées et exécuté** par le joueur optimiste, sauf mention.

* **M07 sabotage** (`quests.toml`) : `site_state` `virus` sur banking-network, `site_state` `backdoor` sur research-lab, `extract` de `prototype_specs` (livre F15), `heat_end_below 60`, et l'objectif **secret et optionnel** `honeypot` (`site_state analyzed` sur research-lab). `on_complete` : trois blocs. Le premier pose `phoenix_state = compromised`, `hub = safehouse`, `sabotage_done`, Phoenix `compromised`, débloque Nexus Insider. Les deux autres sont complémentaires : `when = any[quest s06 completed, objective m07 honeypot]` → `heat_force 65` ; `when = not(...)` → `heat_force 85` (B1 « 65 si S06 ou l'objectif secret »). Le moteur transforme `HeatForced` en `HeatChanged`.
* **M09 / D1** : la décision D1 est en **M10**, pas en M09 (la bible 4.4). M09 (`compromise` ×3, `link aura 1`, chaleur) prépare : son `on_complete` rend le Courtier `available` et l'offre S11. M10 : `talk broker`, `any_of [pay L (when pas de retainer), pay M (when retainer), betray = extract black_ledger, steal = extract shadow-exchange]`, puis `choice d1`. Les choix portent `requires = objective m10 <alternative>` ; leurs effets sont ceux de la bible (`pay` : `core_location_known` ; `betray` : R4Z0R `hostile` ; `steal` : Courtier `hostile`, réputation −S, `heat_floor +20 until m13`). S11 (`broker_retainer`) divise le prix par deux (`tests/language.rs::the_retainer_halves_the_price_of_the_broker`). Le choix de D1 ferme S08 si on n'a pas encore accepté : `available_if = not contact r4z0r hostile` (test `an_offered_contract_is_withdrawn...`).
* **S07 conditionnelle à D2 = SAVE** : `kind = "branch"`, `available_if = flag d2 is save` ; D2 « save » exige `requires = opened f15` (si F15 n'est pas lu, le choix n'est pas offert, et un `DecisionMade` forcé est rejeté et retiré du registre). Si D2 = abandon/turn, S07 reste `UNAVAILABLE` (test) ; elle pose le bit `cutter` (A6).
* **S12 qui peut échouer** : voir § 6.4.
* **M13 boss et M14** : M13 (`compromise aurora-core`, `extract` ×3, `choice d3`) s'ouvre avec `on_open` (chapitre 6, CS09) et se conclut par `settle_ending` + `unlock` de `ending-screen` et `epilogue-montage` ; M14 = `open ending-screen` + `open epilogue-montage` (lecture), la boucle de fin du jeu. Les trois couches du boss sont une affaire du moteur de run, pas du langage : le langage ne voit que `compromise` et `extract`. Les fins de Hardcore GO1-GO3 (grillé) ne passent pas par le langage.

## 8. Limites et points ouverts (la bible contre elle-même ou contre l'audit)

1. **M01 #5 `REACH_TIER(2)` est inatteignable** : la quête qui l'exige est celle qui accorde le palier 2 (audit 3.5 : plus d'XP). Remplacé par `talk echo7` (7 objectifs conservés). Cross-check 13 le laissait prévoir (le tutoriel est à réécrire).
2. **Règle A et colonne « Palier » se contredisent** : avec la règle A (P2 *après* M02), M02 (corp-server-01, P2) et M03 (3 systèmes dont P3) sont injouables. Le spike prend la colonne « Palier » : M01 → 2, M02 → 3, M04 → 4, M06 → 5, M10 → 6.
3. **M03 `REPUTATION(50)` est inatteignable** avec les seules récompenses de quêtes avant M03 (10 + 25 = 35 ; 45 avec S01) puisque la réputation n'est plus achetable. Hypothèse du spike : chaque site rapporte une fois une récompense `S` à la première brèche (`first_breach`, déjà prévu par l'architecture 4.4 « site-<id> »). Le test `the_walker_catches_a_dead_end` supprime cette hypothèse et le joueur optimiste reste bloqué en M03 : l'invariant sert.
4. **R4Z0R** : la bible dit « contact permanent » *récompense* de M03, alors que le premier objectif de M03 est de lui parler, et que le canon la débloque au niveau 2 avec réputation 10 (= la fin de M01). Débloquée à la fin de M01.
5. **M10 objectif 2 (`UNE_DE`) est déjà vrai à l'ouverture** : M04 exige l'extraction du grand livre, donc l'alternative `betray` tient d'emblée ; l'objectif ne contraint jamais le joueur, il ne fait que conditionner les options de D1 (`requires`).
6. **S12 : 60 ou 80 ?** L'objectif dit `HEAT_PEAK_BELOW(60)`, la fiche de Nexus Insider dit « chaleur ≥ 80 pendant S12 », et « avant qu'elle ne soit remplacée » n'a pas d'échéance. Retenu : 60 et « M13 s'ouvre ».
7. **`HEAT_PEAK_BELOW`** : « depuis le début de la quête » (5.1) ou « trace maximale du run » (direction A) ? Retenu : depuis l'ouverture, avec `QuestRestarted` pour les quêtes qui ne peuvent pas échouer ; à trancher avec le moteur de run (« grillé » = rejouer, G3).
8. **Neon Angel « redevient libre si le joueur reprend radio-veille avant M13 »** (3.4) n'est pas exprimable sans un réacteur sur un fait hors quête (non prévu par le langage) : non fait. Si c'est voulu, ajouter un bloc `on_fact` serait la seule extension nécessaire (à décider avant R2).
9. **« −1 de confiance » (D2)** : échelon ou point ? Retenu −10 points. **`broker_deal` (4.4) et `narr.d1` (5.6)** sont le même drapeau. Les 19 contre 22 types d'objectifs de la bible s'expliquent par les alias (`MEET`/`TALK`...).
10. **D2 / M09 dans la consigne** : D1 est en M10 ; D2 en M08 ; D3 en M13. Seule S11 (« à proposer avant M10 ») dépend de l'ordre : le joueur optimiste fait les contrats d'abord, le vrai joueur choisit.
11. **Non couvert** : F07 « jalon hors quête » (palier 5), le service `route` de Ghost Walker et les bonus « une fois par chapitre » (S05, S09), le marché fermé à chaleur 80 (moteur). Quêtes non écrites : S01-S05, S09 (ni décision ni avantage) et S15 (donc l'avantage `blind_spot` n'est jamais posé, ce qui ne change aucune fin). La sérialisation de l'état est faite au lot R2.1 (`persist.rs`, § 1).
12. **Granularité de l'ordre** : l'indépendance à l'ordre est garantie *au sein d'un lot de faits* (le moteur appelle `apply` pour chaque fait en attente, puis `refresh` une fois). Entre deux lots, l'ordre compte par construction : « après l'ouverture » dépend du moment de l'ouverture. Propriétés vérifiées sur des préfixes de parties réelles (`tests/content/props.rs`).

## 9. Ce que le portage (R2.1) a changé par rapport au spike

Le comportement est identique : les 56 tests du spike (dont les 7 de relecture) passent tels quels, transcrits. Ce qui change :

* **Ids** : validés à la création et à la désérialisation (alphabet et longueur de `neon_engine::ids`, soit `[a-z0-9_-]{1,48}`) ; `ContactId` est le type même que les frontends emploient pour un orateur. Le validateur reste plus strict (pas de `-` ni de `_` en tête, `valid_id`) et signale toujours les ids de noms (valeurs de drapeau, ids d'objectifs, groupes de commandes) avec leur ligne.
* **Montants** : `Credits` (0 à 10^9) et `Reputation` (±10^6) remplacent les entiers nus ; l'arithmétique sature, une valeur hors plafond lue dans un fichier ou une sauvegarde est refusée. Les crédits en main (`State::credits`) ne descendent jamais sous zéro et `State::overdrawn` dit si le registre a enregistré plus de dépenses que de gains (le registre ne contrôle pas les prix : c'est à l'appelant de vérifier avant d'émettre `ItemBought` ou `Paid`). La confiance gagnée sature à ±10^6, la chaleur lue est ramenée à 100.
* **Sorties** : `Output` devient `Outcome` ; le futur jeu de campagne (R2.2) en tire des `Event`. `Fact` garde ses 16 faits.
* **Données** : `data/world/*.toml`, embarquées par `build.rs` (`Content::embedded()`) ; les diagnostics portent `data/world/<fichier>.toml:<ligne>`.
* **Sauvegarde** : `State` s'écrit en table à clés textuelles (`persist.rs`) et `State::validate(&Content)` la contrôle au chargement ; l'enveloppe `neon_engine::save` garde sa version.
* **Lectures du hub** : `content::view` (journal par statut avec progression des objectifs comptés, liste des contacts, fin atteinte, chapitre).
