# Spécification : la table unique des commandes

> Phase P1, lot P1.1. Résout la contradiction 9 du [cross-check](../design/CROSS-CHECK.md) (commandes divergentes entre l'audit, la bible, le dossier TUI et l'architecture) et fixe la table que R2 (hub) et R4 (intrusion) implémentent. Le mécanisme (`neon_engine::command::Registry`, noms et alias uniques, commande verrouillée distincte de l'inconnue) existe depuis R1.4a ; ce document dit **quelles commandes** il porte dans le jeu complet. Le vocabulaire est celui de [`glossary.md`](glossary.md).

## 1. Règles

1. **Les noms de commandes ne se traduisent pas** (VOC-5) : `hack`, `shop`, `quests` sont les mêmes en français et en anglais ; seule l'aide d'une ligne est traduite. Ils sont en minuscules ASCII (vérifié par `Registry::issues`), stables d'une version à l'autre (un nom publié ne change pas : on ajoute un alias).
2. **Un contexte, une liste.** Trois contextes : *hub* (la planque et le monde), *intrusion* (un run en cours), *partout*. Une commande d'un autre contexte répond par une raison (« Seulement pendant une intrusion. »), n'est jamais listée par `help` ni complétée : le joueur n'apprend pas ce qui est encore caché, mais n'est jamais laissé devant le message « commande inconnue » quand le nom existe.
3. **Une commande qui n'est pas encore ouverte** se comporte pareil (« Pas encore : parler à R4Z0R. ») sans révéler son effet. L'ouverture est une **donnée** (`data/world/unlocks.toml`, § 6), exprimée dans le langage de conditions des missions : le Rust ne connaît aucune règle d'ouverture.
4. **Numéro ou nom, jamais une chaîne à recopier** (G10). Chaque argument désigne une chose d'une liste ; on la désigne par son **numéro** dans la liste affichée ou par son **nom** (§ 3). Aucun mot secret, aucun texte à recopier.
5. **Les numéros ne bougent pas.** Une ligne d'une liste ne change jamais de numéro tant que la liste existe : une chose épuisée, indisponible ou refusée **reste listée** avec sa raison (« 3. Quantum : niveau 4 requis »), comme la vitrine de R4Z0R du jeu d'origine. Ainsi `buy 3` fait toujours ce que l'écran annonçait.
6. **L'écran n'annonce jamais ce que l'action refuserait.** Une ligne « disponible » est exécutable ; sinon la raison est écrite en mots (`Choice::unavailable` existe déjà).
7. **Une seule action par ligne**, jamais de chaîne `a; b`. Pas de commande qui attend : le temps ne passe qu'aux commandes (P7 de l'audit).
8. **Une erreur dit ce qui s'est passé, puis ce qu'on peut faire** (VOC-4) : « Nœud inconnu : 9. Voir `map`. » Les clés sont `error.*` (§ 4). Les textes d'interface sont neutres (S-2 : ni tu ni vous).
9. **Les commandes de liste ont une forme sans argument qui liste** : `quests`, `net`, `deck`, `contacts`, `messages`, `archives`, `shop`, `laylow`, `load`, `map` (C'est l'exigence de lecteur d'écran « une action = une réponse complète »). **Toute autre commande à argument obligatoire** (`talk`, `accept`, `read`, `decrypt`, `hack`, `link`, `equip`, `unequip`, `buy`, `upgrade`, `probe`, `move`, `breach`, `use`) répond `error.arg.missing` avec son usage et un exemple, sans rien faire : jamais de deviner, jamais de liste implicite.
10. **Les raccourcis clavier de la TUI ont une commande équivalente** (CLA-4) : `F2` ↔ `panel`, `Ctrl+P` ↔ `plain`.

## 2. La table

« Ouverture » dit quand la commande apparaît dans la partie normale ; la colonne « Remplace » renvoie aux 29 commandes du C (audit § 1.1) et à la proposition de l'audit § 3.3.

### 2.1 Partout

| Commande | Alias | Arguments | Effet | Ouverture | Remplace |
|---|---|---|---|---|---|
| `help` | `h` | `[commande]` | liste exactement les commandes ouvertes **dans le contexte** ; avec un nom, son usage et sa ligne d'aide | départ | `help` |
| `status` | `st` | — | fiche : handle, niveau, crédits, Notoriété et bande, réputation par faction, difficulté, indices restants ; **en intrusion** : Trace, cycles, programmes et charges, prévision | départ | `status` |
| `options` | — | — | écran des réglages de présentation (langue, verbosité, ASCII, lecteur d'écran, vitesse du texte, couleurs, difficulté hors Hardcore) | départ | (nouveau) |
| `verbosity` | — | `[brief\|normal\|full]` | affiche ou règle la verbosité ; jamais en dessous de `normal` en mode lecteur d'écran (VRB-3) | départ | (nouveau) |
| `quit` | `exit` | — | quitte ; sauvegarde à l'invite de commande (R-9), confirmation seulement en intrusion | départ | `quit`, `exit` |

### 2.1 bis Commandes de frontend

Ces commandes agissent sur l'affichage ou remplacent la partie, pas sur l'état du jeu : le **frontend** les traite avant le moteur (le plain par son interpréteur de lignes, la TUI par ses menus et raccourcis). Elles sont déclarées dans la **même table** avec le champ `handled_by: HandledBy::Frontend(scope)` (§ 7), où `scope` vaut `AnyFrontend` ou `TuiOnly`, pour apparaître dans `help`, être complétées et vérifiées par `Registry::issues` comme les autres. `help` et la complétion reçoivent les capacités du frontend (`Capabilities { tui: bool }`) et filtrent par `scope` : le plain ne voit jamais `panel` ni `plain`. Le contexte reste celui de la table (`load` est `Hub`, donc refusé en intrusion, les autres `Anywhere`).

| Commande | Alias | Arguments | Effet | Disponible | Raccourci TUI |
|---|---|---|---|---|---|
| `load` | — | `[emplacement]` | sans argument, liste les sauvegardes (autosave, points de contrôle, emplacements) ; avec un argument, charge celle-ci après confirmation ; refusé en intrusion (la partie en cours serait perdue : message « Terminer d'abord l'intrusion ») | plain et TUI, hors intrusion | — |
| `panel` | — | — | ouvre ou ferme le panneau latéral (tiroir au palier compact) | TUI seulement | `F2` |
| `plain` | — | — | bascule vers l'affichage plain en gardant la partie (voir dossier TUI § 4.4) | TUI seulement | `Ctrl+P` |
| `export` | — | `[chemin]` | écrit le transcript de la session (EXP-3) ; le chemin est du type `Path` (§ 3) | plain et TUI | — |

Dans le plain, `panel` et `plain` répondent « Seulement en plein écran. » (même mécanisme que le contexte, § 1 règle 2).


### 2.2 Hub

| Commande | Alias | Arguments | Effet | Ouverture | Remplace |
|---|---|---|---|---|---|
| `quests` | `journal`, `q` | `[quête]` | sans argument, le journal : quêtes actives, offertes (« à accepter »), terminées, avec le chapitre ; avec un argument, les objectifs de la quête | départ | `quests`, `missions` |
| `accept` | — | `<quête offerte>` | accepte un contrat proposé par un contact (statut AVAILABLE → ACTIVE) | première offre | (nouveau) |
| `contacts` | — | — | liste des contacts connus : numéro, nom, relation (inconnu, neutre, amical, de confiance), état (disponible, occupé…) | départ | `contacts` |
| `talk` | `contact` | `<contact>` | engage la conversation : un `Prompt::Choice` de sujets, avec les sujets indisponibles et leur raison | départ (ECHO-7), puis chaque contact à son déblocage | `contact <n\|nom>` |
| `messages` | `inbox` | — | la boîte de réception, non lus marqués | premier message | `messages` |
| `read` | — | `<message>` | lit un message (le marque lu ; peut offrir un contrat) | premier message | `read` |
| `archives` | — | `[document]` | sans argument, la liste des documents et fragments obtenus ; avec un argument, le lit | premier document | (nouveau, remplace le contenu muet des fichiers) |
| `decrypt` | — | `<document>` | déchiffre un document chiffré **choisi dans la liste** ; certains exigent le programme Décryptage ou Quantique, la raison est affichée | premier document chiffré | `decrypt`, `quantumdecrypt` |
| `net` | `sites` | `[site]` | sans argument, la carte du monde : numéro, nom, état (inconnu / connu / percé / grillé), chapitre ; avec un argument, la fiche du site : ce qu'on y sait (familles d'ICE si Intel acquise), butin restant | départ | `scan`, `traceroute`, `analyzedefenses` |
| `hack` | — | `<site>` | lance une intrusion sur un site **connu** : vérifie le deck (« Lancer avec ces 4 programmes ? »), puis passe en contexte intrusion ; en intrusion échouée (« grillé »), la relance avec la pénalité de la difficulté | première mission | `bruteforce`, `exploit`, `backdoor`, `uploadvirus`… |
| `deck` | — | — | le deck : emplacements, programmes équipés, catalogue (niveau d'amélioration, famille, puissance, cycles, bruit, charges), équipements passifs | après la quête du tutoriel (R4Z0R) | (nouveau) |
| `equip` | — | `<programme ou équipement>` | équipe dans un emplacement libre ; refuse avec la raison (emplacements pleins, niveau) | idem `deck` | (nouveau) |
| `unequip` | — | `<programme ou équipement>` | retire du deck | idem `deck` | (nouveau) |
| `shop` | — | — | la vitrine de R4Z0R : numéro, nom, prix, état (disponible, possédé, épuisé, « niveau requis ») | R4Z0R déverrouillé | `shop` |
| `buy` | — | `<objet>` | achète ; refuse avec la raison ; un achat irréversible crée un point de contrôle (R-9) | idem `shop` | `shop` (menu) |
| `upgrade` | — | `<programme>` | améliore un programme du catalogue (niveau 1 → 3) ; prix affiché dans `deck` | idem `shop` | `upgrade_level` (jamais utilisé en C) |
| `laylow` | — | `[service]` | sans argument, les services qui baissent la Notoriété (nom conservé) ; avec un argument, achète le service | quand la Notoriété dépasse « Discret » pour la première fois | `laylow` |
| `tutorial` | — | `[skip\|restart]` | sans argument, redit l'étape courante du tutoriel guidé (ou son état) ; `skip` y met fin ; `restart` le relance depuis la première étape | départ | (nouveau, R2.3b) |
| `link` | — | `<contact>` | renforce d'un niveau (1 à 3) le lien neural avec un compagnon ; seuls les contacts avec qui une quête demande un lien sont proposés (écart de R2.4, voir § 11) | chapitre 4 | `neuralsync` |
| `hint` | — | — | un indice d'ECHO-7 sur l'objectif actif ; décompte selon la difficulté ; jamais de solution complète | départ (nombre selon la difficulté) | (nouveau) |
| `save` | — | `[emplacement 1-9]` | sans argument, point de contrôle manuel dans l'emplacement libre le plus ancien ; avec, dans cet emplacement ; jamais en pleine intrusion (§ 5) | départ | `save` |

### 2.3 Intrusion

| Commande | Alias | Arguments | Effet | Ouverture | Remplace |
|---|---|---|---|---|---|
| `map` | — | — | le graphe : nœuds numérotés et nommés, liens, ICE connues (famille, force, progression), patrouilles et leur prochaine case, nœud courant, butin | toujours en intrusion | `traceroute`, `analyzedefenses` |
| `probe` | — | `<nœud>` | révèle l'ICE et le butin d'un nœud adjacent (ou la famille d'une ICE) ; le Compagnon en offre un gratuit par tour | toujours en intrusion | `traceroute`, `analyzedefenses` |
| `move` | `go` | `<nœud>` | se déplace vers un nœud **adjacent** ouvert | toujours en intrusion | (relais du C) |
| `breach` | — | `<nœud> <programme>` | attaque l'ICE d'un nœud avec un programme offensif : ajoute sa puissance à la progression (règle de famille) ; coût en cycles, bruit en Trace | toujours en intrusion | `bruteforce`, `exploit`, `aihack`, `advhack`… |
| `use` | — | `<programme> [nœud]` | lance un programme utilitaire : Cloak, Spoof, Ghost, Overclock, Backdoor ; l'argument sert aux programmes qui visent un nœud | toujours en intrusion | `stealth`, `laylow`, `neuralsync`, `backdoor` |
| `loot` | — | `[nœud]` | ramasse le butin du nœud courant (ou désigné s'il est ouvert) ; une fois | toujours en intrusion | (fichiers du C) |
| `end` | — | — | termine le tour : la prévision se joue, la Trace ambiante s'ajoute, les patrouilles avancent | toujours en intrusion | (nouveau) |
| `jackout` | — | — | quitte l'intrusion avec le butin ramassé ; demande confirmation si le butin visible n'est pas ramassé | toujours en intrusion | (nouveau) |
| `undo` | — | — | rend le dernier tour ; réglé par la difficulté (illimité en Histoire, 3 par intrusion en Normal, aucun en Expert et Hardcore) ; la liste dit « 2 restants » | toujours en intrusion | (nouveau) |
| `skip` | — | — | résolution automatique de l'intrusion (`AutoResolve`) ; seulement en difficulté Histoire, marqué dans la sauvegarde ; confirmation | difficulté Histoire | (nouveau, G8) |

`status` et `help` fonctionnent aussi en intrusion (§ 2.1). **Choix de l'audit non repris** : l'audit liste `cloak` et `spoof` comme commandes dédiées ; seuls deux utilitaires sur quatre auraient une commande, ce qui est incohérent. Tous les utilitaires passent par `use <programme>` (`use cloak`). Si l'expérience montre que c'est trop long à taper, R4 ajoutera des alias de **programme** (la saisie `cloak` ouvre le programme) sans changer la table.

### 2.4 Désignation d'un nœud, d'un programme, d'un site

- **Nœud** : numéro (1 à 8) affiché par `map`, ou nom (« passerelle », « coffre »). Les lettres de l'exemple de l'audit (P, F, A, C) sont **abandonnées** : une lettre est ambiguë à la voix et collisionne avec les noms.
- **Programme** : numéro dans `deck`, ou nom (« brute », « exploit »). Les noms de programmes sont des noms propres du jeu (« Brute-Force ») ; on tape leur **premier mot**, sans casse ni accent.
- **Site** : numéro dans `net`, ou identifiant (`corp-server-01`).

## 3. Résolution d'un argument

Un argument est lu en quatre étapes, la même pour toutes les commandes (une seule fonction du moteur, testée une fois) :

1. **Numéro** : un entier décimal ; il désigne la ligne de la liste du même genre. Hors liste : `error.arg.unknown`.
2. **Nom exact** : le nom canonique ou un alias, casse et accents ignorés (`Passerelle`, `passerelle`, `PASSERELLE` ; `élevé` = `eleve`).
3. **Préfixe unique** : un début de nom qui ne convient qu'à une chose (`pass` → passerelle).
4. Sinon : **ambigu** (liste des candidats, numérotés) ou **inconnu**.

La complétion (`TAB`) propose les choix des étapes 2 et 3 qui sont **ouverts** ; elle ne propose jamais une chose verrouillée ou inconnue (pas de spoiler, I7). **Exception unique, le chemin** (`ArgKind::Path`, seulement pour `export`) : le reste de la ligne, tel quel, espaces conservés, sans normalisation de casse ni d'accents, sans numéro ni complétion de nom (le frontend complète les fichiers). Les autres arguments à plusieurs mots n'existent pas : un nom de chose tient en un mot de saisie (le nom affiché peut en avoir plusieurs, la saisie en a un : « Stealth Module v2.0 » se tape `stealth`).

## 4. Erreurs

| Clé | Quand | Forme |
|---|---|---|
| `error.unknown_command` | le nom n'existe dans aucun contexte | « Commande inconnue : bogus. Voir `help`. » |
| `error.locked` | le nom existe mais n'est pas ouvert (raison fournie par l'ouverture) | « Pas encore : {raison}. » |
| `error.wrong_context.run`, `.hub`, `.tui` | commande d'un autre contexte, ou que le frontend ne sait pas faire | « Seulement pendant une intrusion. » / « Pas pendant une intrusion. » / « Seulement en plein écran. » |
| `error.arg.missing` | argument obligatoire absent | « Argument manquant. Usage : `breach <nœud> <programme>`. Exemple : `breach 3 brute`. » (l'usage est la clé `help.<commande>.usage`) |
| `error.arg.unknown` | numéro ou nom hors liste | « Inconnu : nœud 9. Voir `map`. » (le genre est le terme du glossaire ; la majuscule initiale attend le marqueur `{nom^}`) |
| `error.arg.ambiguous` | préfixe qui convient à plusieurs choses | « « s » convient à plusieurs programmes : 2. spoof, 3. stealth. » |
| `error.arg.unavailable` | la chose existe mais est refusée | « stealth est indisponible : cycles insuffisants. » (la raison est celle de la ligne de la liste, propre à l'action : cycles insuffisants, nœud non adjacent, crédits manquants…) |

Les erreurs sont des événements de rôle `Error`, donc toujours `Essential` (VRB-2). Chaque clé a sa variante `@sr` si elle contient un symbole ambigu.

## 5. Sauvegarde et contextes

- `save` n'est ouvert **qu'au hub** (cohérent avec R-9 : une partie se sauvegarde à l'invite de commande, jamais au milieu d'un tour) ; l'autosave se fait à la fin de chaque commande du hub et **à la fin de chaque tour** d'intrusion (la reprise en pleine intrusion revient au début du tour, jamais au milieu).
- Le point de contrôle est créé automatiquement au début d'une mission, avant une décision D1-D3 et avant un achat irréversible.
- `quit` en intrusion demande confirmation (« Quitter l'intrusion en cours ? La partie reprendra au début du tour. »).

## 6. Ouverture des commandes (donnée)

`data/world/unlocks.toml` (R2) associe chaque commande à une condition du langage de missions et à la clé de sa raison :

```toml
[[unlock]]
command = "shop"
when = { contact_state = { id = "r4z0r", is = "available" } }
reason = "unlock.shop"          # « Pas encore : trouver R4Z0R. »
```

Ouverture initiale proposée (la fin de chaque ligne est la quête qui l'ouvre ; tout peut se régler dans la donnée sans toucher au code) :

| Étape | S'ouvre |
|---|---|
| départ | `help`, `status`, `save`, `options`, `verbosity`, `quit`, `quests`, `contacts`, `talk` (ECHO-7), `hint`, `net` |
| M01 (tutoriel), à la première mission | `hack`, puis en intrusion tout le jeu de commandes d'intrusion |
| fin de M01 | R4Z0R : `shop`, `buy`, `deck`, `equip`, `unequip`, `upgrade` |
| premier message | `messages`, `read` |
| premier document | `archives`, `decrypt` |
| première offre de contrat | `accept` |
| Notoriété au-dessus de « Discret » une fois | `laylow` (écart de R2.2b : voir § 11) |
| chapitre 4 (M08 ouverte) | `link` (écart de R2.4 : § 11) |
| difficulté Histoire | `skip` |

Le tutoriel (R2) enseigne dans cet ordre `help`, `quests`, `talk`, `net`, `hack`, `map`, `breach`, `end`, `jackout` (les quatre dernières attendent R4 ; le tutoriel du hub, lot R2.3b, enseigne `help`, `status`, `quests`, `talk`, `net`, `hack`, `laylow`, `shop`, `buy`, `save`, `quit`, voir le § 11) ; il ne dépend d'aucune commande retirée (`bruteforce localhost`, `laylow` comme premier geste de couverture, le niveau 2 par `scan` : contradiction 13).

## 7. Ce que R2 change dans le code

**Fait au lot R2.2a** (`neon_engine::command`, mécanisme et tests ; la table du jeu complet est au lot R2.2b). Le registre de R1.4a n'avait ni contexte, ni argument typé, ni usage. `CommandSpec` porte maintenant :

```rust
pub struct CommandSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub help: &'static str,        // « help.<nom> » : une ligne ; l'usage est « help.<nom>.usage »
    pub context: Context,          // Anywhere | Hub | Run
    pub handled_by: HandledBy,     // Engine | Frontend(Scope), Scope = AnyFrontend | TuiOnly
    pub args: &'static [ArgSpec],  // { kind: ArgKind, optional: bool }
}
pub enum ArgKind { Number, Quest, Contact, Message, Document, Site, Node, Program, Item, Slot, Service, Word(&'static [&'static str]), Path }
pub struct Capabilities { pub tui: bool }   // Capabilities::PLAIN, Capabilities::TUI
```

Le jeu fournit un `Resolver` : `list(kind, command) -> Listing { see, rows }`, où chaque `Row { id, names, available: Result<(), Text> }` est une ligne de la liste affichée (la ligne `n` est le numéro `n` ; une chose indisponible **reste** dans la liste avec sa raison ; `available` peut dépendre de la commande). Le moteur lit seul les nombres, les mots fixes et les chemins. Les mots en trop après les arguments déclarés sont ignorés.

```rust
registry.parse(line, context, capabilities, availability, &resolver) -> Lookup
registry.complete(prefix, context, capabilities, availability) -> Vec<String>
registry.complete_args(line, context, capabilities, availability, &resolver) -> Vec<String>
registry.help(title, columns, context, capabilities, availability) -> Event
enum Lookup { Empty, Found { spec, args: Vec<ArgRef> }, Locked { spec, reason },
              WrongContext { spec, reason }, BadArg { spec, error: ArgError }, Unknown(String) }
enum ArgRef { Number(u32), Word(&'static str), Path(String), Listed { kind, number, id } }
enum ArgError { Missing { usage }, Unknown { kind, word, see }, Ambiguous { kind, word, candidates },
                Unavailable { name, reason } }   // ArgError::text() : les clés error.arg.*
```

L'ordre des réponses est : nom inconnu, mauvais contexte ou frontend incapable (`error.wrong_context.run|hub|tui`), verrouillée, puis les arguments de gauche à droite. Le frontend qui ne gère pas `panel` et `plain` (`Capabilities::PLAIN`) les voit répondre « Seulement en plein écran. ». `help`, `complete` et `complete_args` ne listent et ne proposent que ce qui est ouvert, du contexte courant et possible pour le frontend (jamais une chose verrouillée, fermée ou inconnue).

`Registry::issues` contrôle : noms et alias bien formés et uniques, mot fixe non vide, argument facultatif jamais avant un obligatoire, chemin en dernier, aucune commande d'intrusion listée au hub. `Registry::catalog_issues(&Catalog)` contrôle séparément que chaque `help.<nom>` existe et que toute commande qui a des arguments a son `help.<nom>.usage`. **Fait en R2.2b** : aucun nom de commande n'est un mot interdit du glossaire et tout `ArgKind` utilisé a un résolveur (tests de `neon_engine::campaign`). **Reste** : le frontend ne transmet pas encore ses `Capabilities` au jeu (voir § 11).

## 8. Tests exigés (I7 et suivants)

1. Tout ce que `help` affiche s'exécute ; tout ce qui s'exécute et est ouvert est dans `help` (R1.4a, étendu aux contextes).
2. `Locked` ≠ `Unknown` ≠ `WrongContext` ; une commande fermée n'est ni listée ni complétée.
3. Noms et alias uniques, minuscules ASCII ; un alias n'est complété que si aucun nom officiel ne convient.
4. **Numérotation stable** : pour toute séquence de commandes, une ligne ne change pas de numéro tant que sa liste existe (propriété `proptest` sur `shop`, `deck`, `net`, `quests`).
5. Parser d'argument : propriétés (numéro valide, nom exact, préfixe unique, ambigu, inconnu, accents et casse ignorés) ; jamais de panique sur une entrée arbitraire (I13).
6. Chaque commande a un transcript `insta` en français et en anglais pour sa forme sans argument et sa forme d'erreur.
7. Le tutoriel joué (joueur optimiste) n'utilise que des commandes ouvertes à ce moment-là.

## 9. Correspondance complète avec le C

| Commande du C | Devient |
|---|---|
| `scan` | `net` (macro) ; `probe` (en intrusion) |
| `bruteforce`, `exploit`, `aihack`, `advhack`, `temporalhack`, `quantumdecrypt` | `breach <nœud> <programme>` (programmes Brute-Force, Exploit, Compagnon, Quantique…) |
| `backdoor`, `uploadvirus` | programmes Backdoor et Virus (`use` / `breach`) |
| `traceroute`, `analyzedefenses` | `probe`, `map`, `net <site>` |
| `decrypt` | `decrypt <document>` (choix dans la liste) |
| `stealth`, `stealthmode`, `aiassist` | supprimés ; Cloak, Compagnon, Overclock (`use`) |
| `neuralsync` | `link <contact>` au hub (R2.4, en attendant l'emplacement Compagnon de R4) |
| `socialeng` | programme Ingénierie sociale |
| `shop` | `shop`, `buy`, `upgrade` |
| `laylow` | `laylow` (services de Notoriété) |
| `quests`, `contacts`, `contact`, `messages`, `read` | `quests`, `contacts`, `talk`, `messages`, `read` |
| `status`, `help`, `save`, `quit`/`exit`, `clear` | idem ; `clear` supprimé (la TUI défile, le plain n'efface pas) |

## 10. Décisions

Validées par le propriétaire le 8 octobre 2026 (S-3 de [`DECISIONS.md`](../design/DECISIONS.md) § 5) : `hack <site>` comme seule porte d'entrée d'une intrusion ; `use <programme>` pour tous les utilitaires ; nœuds désignés par numéro ou nom (pas de lettre) ; `net` pour la carte du monde et `map` pour le graphe (règle du cross-check) ; `talk` officiel et `contact` en alias ; `journal` en alias de `quests` ; `save` réservé au hub ; autosave à la fin de chaque tour d'intrusion.

## 11. Mise en œuvre du hub (lot R2.2b) : ce qui s'écarte du texte ci-dessus

La table du hub est déclarée dans `neon_engine::campaign` (24 noms). Les commandes de l'intrusion, `deck`, `equip`, `unequip`, `upgrade`, `skip`, `options` et `verbosity` n'y sont pas : les premières attendent le moteur de run (R4), les deux dernières les réglages de présentation, que la TUI traitera (R3). La question d'`equip` (l'audit n'avait aucune commande de ce nom) est **sans objet jusqu'à R4**.

1. **Alias `?`** : la règle 1 (noms et alias en minuscules ASCII, vérifiée par `Registry::issues`) l'interdit. L'alias de `help` est supprimé de la spécification : `help` et `h`.
2. **Numéros stables** : les listes qui grossissent avec le jeu (`quests`, `contacts`, `messages`, `archives`) sont numérotées **dans l'ordre où le joueur a rencontré les choses** (liste à ajout seul, enregistrée dans la sauvegarde), jamais dans l'ordre des données : une quête qui s'ouvre plus tard ne décale pas les autres. Les listes à contenu fixe (`net`, `shop`, `laylow`, `save`) gardent l'ordre des données et listent tout, le non-disponible avec sa raison ; `net` s'arrête au dernier site connu et compte le reste (« 15 autres sites encore inconnus ») au lieu d'aligner des `???`, les numéros restant ceux de la liste entière.
3. **`net`** : la colonne « chapitre » de la table est **« niveau »** (le niveau requis du site) : le catalogue ne donne pas de chapitre aux sites. Un site est *inconnu* tant que son niveau ou son relais manque, *connu* ensuite, *percé* quand il a été compromis ; l'état *grillé* attend le moteur de run.
4. **`laylow` s'ouvre avec M01**, et non quand la Notoriété dépasse « Discret » : le dernier objectif de M01 est « utiliser une commande de couverture » ; ouvrir `laylow` plus tard rendrait M01 impossible à finir. La fenêtre « première fois au-dessus de Discret » n'existe pas dans le langage de conditions (aucune condition ne lit la chaleur hors objectif).
5. **`shop` et `buy` restent ouverts si R4Z0R devient hostile** (D1, `betray`) : la fermer ferait perdre l'accès aux objets des niveaux suivants (la puce quantique ouvre `doc_vault`). La boutique se ferme en revanche à Notoriété « Chassé » (80), raison écrite dans la liste.
6. **`save` sans argument** écrit l'emplacement 1 : le moteur ne sait pas quel emplacement est libre ou le plus ancien (c'est le dossier de sauvegardes du frontend qui le sait).
7. **`hack`** : la confirmation ne vérifie pas de deck (il n'existe pas avant R4) ; l'intrusion est résolue aussitôt (`AutoResolve`, voir R-15). `skip` n'existe pas : tout `hack` est déjà automatique.
8. **Payer et décider** ne sont pas des commandes : un objectif `pay` se règle dans la conversation du donneur de la quête (`talk`, entrée « Payer N crédits »), une décision D1-D3 s'ouvre depuis la conversation de celui qui la porte (entrée « Décider »), avec un point de contrôle avant le choix.
9. **Commandes du frontend** (`load`, `panel`, `plain`, `export`) : déclarées dans la table (listées par `Registry`, vérifiées par `Registry::issues`), mais **aucun frontend ne les prend encore en charge** : le jeu les ferme (« Pas encore : l'interface ne le propose pas encore. ») au lieu de les lister ; `panel` et `plain` répondent « Seulement en plein écran. ». Quand R3 les câblera, il suffira de les rouvrir (une ligne de `availability`) et de transmettre les `Capabilities`.
10. **`accept`** s'ouvre à la première offre, S06 (règle de `unlocks.toml`) ; **`messages`/`read`** et **`archives`/`decrypt`** à la fin de M01 (premier message, niveau 2 qui montre le premier document chiffré).
11. **Textes** : les noms de commandes dans les textes sont des noms propres du jeu ; les marques de substitution des usages (`<quête>`, `<contact>`) sont écrites dans la langue de l'interface, car un usage est un texte sans arguments.
12. **`tutorial [skip|restart]`** (lot R2.3b) : le nom `skip` de la table de l'intrusion (§ 2.3) reste réservé à R4 ; le tutoriel guidé se coupe donc par `tutorial skip`, ou en refusant l'offre qui le propose après le prologue. Une étape se fait en exécutant sa commande, dans n'importe quel ordre ; l'indication de l'étape courante n'est dite qu'au prompt de commande, une seule fois, et seulement quand la commande de l'étape est ouverte (la boutique attend la fin de M01, par la règle de `unlocks.toml`). Dans le plain, `skip` tapé à un « Entrée pour continuer » saute le reste du prologue (Échap dans la TUI).
13. **`link <contact>`** (lot R2.4, décision R-17) : le langage de missions demande des niveaux de lien neural (objectifs `link`, S07, M09, S10, M11, S16) et a le fait `LinkChanged`, mais rien ne le produisait avant l'emplacement Compagnon du deck (R4) : le joueur optimiste de bout en bout s'arrêtait à M09. En attendant, `link <contact>` monte le lien d'un niveau par commande (de 0 à 3), gratuitement, pour les seuls contacts avec qui une quête demande un lien (la donnée des quêtes est la seule source : ECHO-7 et AURA aujourd'hui). La commande s'ouvre au chapitre 4 (`unlocks.toml`). Le run tactique donnera les mêmes faits (R4) : les quêtes ne changent pas.
14. **Un paiement reste proposé quand une autre voie a réglé l'objectif** (lot R2.4) : M10 règle son deuxième objectif par l'une de quatre voies (payer, payer à demi-tarif, trahir avec le registre, voler) et la décision D1 n'offre « payer » qu'à qui a payé. Le registre pris en M04 réglait déjà l'objectif et retirait l'entrée « Payer » du menu du Courtier : payer était impossible par les commandes. Les alternatives d'un `any_of` non accomplies restent maintenant proposées même quand l'objectif est réglé.
