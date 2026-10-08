# Spécification : l'écran d'intrusion et sa linéarisation

> Phase P1, lot P1.3. Résout la contradiction 10 du [cross-check](../design/CROSS-CHECK.md) : aucun dossier ne spécifiait l'écran d'intrusion (le « run ») ni la façon de le lire à voix haute. Le modèle de règles est celui de l'audit (§ 3.3) ; le vocabulaire, celui du [glossaire](glossary.md) ; les commandes, celles de [`commands.md`](commands.md) § 2.3. Les chiffres de règles sont ceux de départ de l'audit : ils se règlent avec le solveur et `neon-sim`, jamais ici.

## 1. Principe

> **L'écran d'intrusion n'a pas d'information propre.** Tout ce que la TUI montre dans son panneau est dit en texte dans le terminal, et tout ce que le terminal dit se déduit d'une suite de **faits**, un fait par ligne. Le plain est la référence ; la TUI ne montre rien de plus que ce que le texte dit (P6).

Quatre conséquences :

1. **Une ligne = un fait**, dans un ordre fixe (§ 3). Rien n'est « dessiné » : pas de graphe en caractères dans le plain, pas de curseur déplacé.
2. **La prévision et la résolution sont la même fonction** (I10). Ce que la ligne « Prévision » annonce, la fin du tour le fait, au chiffre près ; l'aléa (butin, variantes de texte) n'y entre pas.
3. Chaque fait existe en **trois niveaux d'importance** (§ 7) : la verbosité `brief` suffit pour jouer.
4. Le panneau de la TUI (§ 6) est une **vue** (`RunView`) lue à la demande, pas un flux d'événements : le redimensionnement, le tiroir et la reprise après chargement ne perdent rien.

## 2. Le contrat moteur ↔ frontends

### 2.1 `View`

`Game::view()` rend aujourd'hui une structure plate (joueur, jauges, objectifs). R2 la remplace par une énumération :

```rust
pub enum View {
    Hub(HubView),
    Run(RunView),
}

pub struct RunView {
    pub site: Text,                  // « MegaCorp Industries, serveur de paie »
    pub turn: u32,
    pub trace: GaugeReading,         // valeur, max 100, bande en mot
    pub cycles: Cycles,              // { left, max }
    pub nodes: Vec<NodeView>,        // dans l'ordre des numéros
    pub patrols: Vec<PatrolView>,
    pub deck: Vec<ProgramView>,
    pub forecast: Vec<Text>,         // les faits de § 4, déjà en Text
    pub forecast_total: i32,         // variation de Trace annoncée si on termine le tour
    pub undo_left: Undo,             // Unlimited | Left(n) | None
    pub loot_held: Vec<Text>,
}

pub struct NodeView {
    pub id: NodeId,                  // numéro affiché, stable pendant l'intrusion
    pub name: Text,                  // ≤ 10 colonnes (§ 6.3)
    pub state: NodeState,            // Here | Open | Closed | Unknown
    pub defense: Option<DefenseView>,// None = pas de défense ; connue ou non
    pub links: Vec<NodeId>,
    pub loot: LootState,             // None | Visible | Taken
}

pub struct DefenseView { pub family: Term, pub strength: u8, pub progress: u8, pub known: bool }
pub struct PatrolView { pub id: u8, pub at: NodeId, pub next: NodeId, pub neutralized_turns: u8 }
pub struct ProgramView { pub name: Text, pub family: Term, pub power: u8, pub cycles: u8, pub noise: u8, pub charges: Charges }
```

`Cycles`, `Undo`, `Charges` sont des types du moteur, jamais des entiers nus. Un nœud ou une défense **inconnus** (avant `probe`, sans Intel) ont `known: false` : le texte dit « inconnue » et le panneau « ? ». La `View` est de la donnée ; **chaque champ a son équivalent texte** (§ 3-4), c'est ce que RUN-1 vérifie.

### 2.2 `Event`

Le contrat de R1 (`Message`, `Decor`, `Screen`, `Changed`, `Break`) suffit, avec deux changements, prévus par le cross-check 10 :

- `Gauge` gagne `Notoriety` ; `Changed { gauge, from, to, band }` sert aussi à la Notoriété. Les cycles **ne** sont **pas** une jauge : leur restant s'écrit à la fin de chaque action (§ 3) ;
- `Event::Break` n'est jamais émis au milieu d'un tour : il sépare les tours.

`Screen(Table)` porte `map`, `status` (section « programmes ») et le rapport de fin. Aucun nouveau type d'événement n'est nécessaire : un fait est un `Message` dont le **rôle** (`System`, `Alert`, `Reward`, `Error`) et l'**importance** sont fixés par § 7, et dont la clé est dans la famille `run.*` (§ 8).

### 2.3 `Prompt`

Dans une intrusion, l'invite est `Prompt::Command` (contexte *intrusion* de la table de commandes). Deux confirmations existent : `jackout` quand du butin visible n'est pas ramassé, et `quit`. Après une intrusion terminée (réussie ou grillée), l'invite revient au hub ; une intrusion grillée propose `hack <site>` pour la rejouer (§ 9).

## 3. Ordre des lignes

### 3.1 Après une action du joueur

Toujours dans cet ordre, une ligne par fait, les absents omis :

1. le **résultat** de l'action (`run.use.*`, `run.breach.*`, `run.move`, `run.loot`, `run.probe`) ;
2. si un nœud change d'état, la ligne `run.node.open` ;
3. la variation de **Trace** (`Changed`, rôle `System`) ; un changement de bande ajoute un `Alert` (§ 7) ;
4. la **projection** `run.projection` **si, et seulement si, la variation de Trace annoncée pour la fin du tour a changé** (« Fin de tour maintenant : Trace +2 (0 → 2, calme). ») ;
5. le **reste de cycles** (`run.cycles`) quand l'action en a coûté ; à 0, la ligne devient « Plus de cycle : `end` termine le tour. ».

Une action refusée (`error.arg.unavailable`) n'écrit que son erreur, sans les lignes 3 à 5.

### 3.2 Fin de tour (`end`)

Dans l'ordre :

1. les **mouvements** des patrouilles, par numéro croissant (`run.end.patrol_moves`) ;
2. les **scans**, par numéro de patrouille croissant : `run.end.scan` (avec la Trace ajoutée) ou `run.end.scan_blocked` (annulé par Cloak, par une patrouille neutralisée…) ;
3. la **Trace ambiante** (`run.end.ambient`) ;
4. la variation de Trace **totale** (`Changed`) et l'éventuel changement de bande ;
5. l'entrée dans le **tour suivant** : `Event::Break`, puis `run.turn.start` ;
6. la **prévision détaillée** (§ 4).

### 3.3 Les trois cas qui terminent l'intrusion

`jackout` (rapport, § 9) ; **Trace à 100** (« grillé », rôle `Alert(Danger)`, rapport, § 9) ; `skip` en difficulté Histoire (le plan optimal du solveur est appliqué ; mêmes lignes de rapport, marquées « automatique »).

## 4. La prévision

Elle est calculée par `RunState::forecast()`, **la même fonction** que celle qui résout la fin de tour (`RunState::end_turn()` en consomme le résultat) : un test applique les deux sur chaque état atteint par un bot et exige l'égalité des faits (I10).

**Contenu**, dans l'ordre de § 3.2, une ligne par fait :

| Fait | Clé | Exemple (anglais) |
|---|---|---|
| mouvement d'une patrouille | `run.forecast.patrol_moves` | « Forecast: Patrol 1 moves to Archives (3). » |
| scan | `run.forecast.scan` | « Forecast: Patrol 1 scans Gateway (1): Trace +5, unless Cloak is active. » |
| scan déjà annulé | `run.forecast.scan_blocked` | « Forecast: Patrol 1 scans Firewall (2): cancelled by Cloak. » |
| Trace ambiante | `run.forecast.ambient` | « Forecast: ambient Trace +2. » |

La phrase « sauf si Cloak est actif » n'apparaît que si Cloak est dans le deck avec au moins une charge : le jeu n'annonce pas ce que le joueur ne peut pas faire. **Horizon** : un tour en Normal, Expert et Hardcore ; deux tours en Histoire (la seconde série est introduite par `run.forecast.next_turn`) ; en Expert et Hardcore, une patrouille n'est annoncée que si son nœud d'arrivée ou son scan a été révélé par `probe` (audit § 3.8).

**Pas de prévision de l'aléa.** Le butin et les variantes de texte ne figurent jamais dans la prévision ; toute règle future qui rendrait un scan aléatoire casserait le contrat et devra passer par une décision G2.

**Au brief.** La prévision détaillée est d'importance `Normal` ; une ligne résumée (`run.forecast.total` : « Forecast: Trace +7 at the end of the turn. ») est `Essential`. Le total se lit aussi dans `status`.

## 5. Un tour entier, en trois présentations

Les chiffres sont ceux de la mission d'exemple : site « MegaCorp Industries, serveur de paie », 5 nœuds (1 Passerelle, 2 Pare-feu, 3 Archives, 4 Coffre, 5 Caméras), liens 1-2, 1-3, 2-3, 3-4, 3-5, 4-5, défenses Pare-feu = Réseau 2, Coffre = Réseau 4, Caméras = Gardien IA 3, une patrouille d'itinéraire 5, 3, 2, 3, 5…, un scan = +5 Trace si la patrouille est sur le nœud du joueur ou à côté, ambiante +2, difficulté Normal (3 cycles), deck : Brute-Force (Réseau 2, 1 cycle, bruit 5), Exploit (Réseau 3, 2 cycles, bruit 2, 2 charges), Cloak (1 cycle, 2 charges). Les valeurs sont **illustratives** : ni l'équilibre ni l'écriture ne sont figés.

### 5.1 Plain, anglais, verbosité `normal`

```
Run: MegaCorp Industries, payroll server. Intel: every defense is known.
Turn 1: Trace 0/100 (calm), cycles 3/3.
Forecast: Patrol 1 moves to Archives (3).
Forecast: Patrol 1 scans Gateway (1): Trace +5, unless Cloak is active.
Forecast: ambient Trace +2.
> map
Map: payroll server
[1] Gateway - you are here - no defense - links 2, 3
[2] Firewall - closed - Network 2, progress 0/2 - links 1, 3
[3] Archives - open - no defense - links 1, 2, 4, 5
[4] Vault - closed - Network 4, progress 0/4 - links 3, 5
[5] Cameras - closed - AI Guardian 3, progress 0/3 - links 3, 4
Patrol 1: at Cameras (5), then Archives (3).
> use cloak
Cloak active: the next scan is cancelled (1 charge left).
End of turn now: Trace +2 (0 → 2, calm).
Cycles 2/3.
> breach 2 brute
Brute-Force on Firewall (2): progress 2/2.
Firewall (2) is open.
Trace +5 (0 → 5, calm).
Cycles 1/3.
> move 2
Position: Firewall (2).
Cycles 0/3. No cycle left: type end.
> end
Patrol 1 moves to Archives (3).
Patrol 1 scans Firewall (2): cancelled by Cloak.
Ambient Trace +2.
Trace +2 (5 → 7, calm).

Turn 2: Trace 7/100 (calm), cycles 3/3.
Forecast: Patrol 1 moves to Firewall (2).
Forecast: Patrol 1 scans Firewall (2): Trace +5, unless Cloak is active.
Forecast: ambient Trace +2.
> move 3
Position: Archives (3).
Cycles 2/3.
> use cloak
Cloak active: the next scan is cancelled (0 charges left).
End of turn now: Trace +2 (7 → 9, calm).
Cycles 1/3.
> breach 4 brute
Brute-Force on Vault (4): progress 2/4.
Trace +5 (7 → 12, calm).
Cycles 0/3. No cycle left: type end.
> end
Patrol 1 moves to Firewall (2).
Patrol 1 scans Archives (3): cancelled by Cloak.
Ambient Trace +2.
Trace +2 (12 → 14, calm).

Turn 3: Trace 14/100 (calm), cycles 3/3.
Forecast: Patrol 1 moves to Archives (3).
Forecast: Patrol 1 scans Archives (3): Trace +5.
Forecast: ambient Trace +2.
> breach 4 exploit
Exploit on Vault (4): progress 4/4.
Vault (4) is open.
Trace +2 (14 → 16, calm).
Cycles 1/3.
> move 4
Position: Vault (4).
Cycles 0/3. No cycle left: type end.
> end
Patrol 1 moves to Archives (3).
Patrol 1 scans Vault (4): Trace +5.
Ambient Trace +2.
Trace +7 (16 → 23, calm).

Turn 4: Trace 23/100 (calm), cycles 3/3.
Forecast: Patrol 1 moves to Cameras (5).
Forecast: Patrol 1 scans Vault (4): Trace +5.
Forecast: ambient Trace +2.
> loot
[Reward] Payroll ledger: +400 credits.
Cycles 2/3.
> jackout
You jack out of MegaCorp Industries with the payroll ledger.
Run report
[1] Turns - 4
[2] Final Trace - 23/100 (calm)
[3] Loot - 400 credits, 1 document
[4] Notoriety +5 (12 → 17, Discreet)
```

Deux choses à lire dans cet exemple. À la turn 3, la phrase « unless Cloak is active » a disparu : le deck n'a plus de charge de Cloak, donc le jeu ne la propose pas. Et le **chiffre de la prévision est exact** : au début du tour 3 elle annonce +5 et +2, et la fin du tour apporte +5 et +2 (16 → 23).

### 5.2 Plain, français, début du tour 1 et `use cloak`

```
Intrusion : MegaCorp Industries, serveur de paie. Intel : toutes les défenses sont connues.
Tour 1 : Trace 0/100 (calme), cycles 3/3.
Prévision : la patrouille 1 va en Archives (3).
Prévision : la patrouille 1 scanne la Passerelle (1) : Trace +5, sauf si Cloak est actif.
Prévision : Trace ambiante +2.
> use cloak
Cloak actif : le prochain scan est annulé (1 charge restante).
Fin de tour maintenant : Trace +2 (0 → 2, calme).
Cycles 2/3.
```

### 5.3 Lecteur d'écran et `--ascii` combinés, même passage

```
Run: MegaCorp Industries, payroll server. Intel: every defense is known.
Turn 1. Trace 0 out of 100, level calm. 3 of 3 cycles.
Forecast: Patrol 1 moves to Archives, node 3.
Forecast: Patrol 1 scans Gateway, node 1: Trace plus 5, unless Cloak is active.
Forecast: ambient Trace plus 2.
> use cloak
Cloak active: the next scan is cancelled. 1 charge left.
End of turn now: Trace plus 2, from 0 to 2, level calm.
2 of 3 cycles.
> map
Map: payroll server
1. Node: Gateway; State: you are here; Defense: none; Links: 2 and 3
2. Node: Firewall; State: closed; Defense: Network 2, progress 0 of 2; Links: 1 and 3
```

Les variantes `@sr` suppriment les parenthèses (« (3) » devient « , node 3 »), épellent « plus » et « from … to », et séparent les listes par « and ». Le mode `--ascii` ne change ici que `→`.

## 6. La TUI : le panneau d'intrusion

Le panneau remplace le panneau du hub (Trace, quêtes, réseau, inventaire) pendant une intrusion ; il est dessiné **uniquement** depuis `RunView`.

### 6.1 Disposition à 100×28 (palier complet)

```
 Case · 197 ¢ · Trace 16/100 (calme) · Intrusion · Tour 3
╭ Terminal ──────────────────────────────────────────────────────╮╭ Trace ───────────────────────╮
│La patrouille 1 va en Pare-feu (2).                             ││Trace 16/100 (calme)          │
│Patrouille 1 scanne Archives (3) : annulé par Cloak.            ││Cycles 1/3  ●○○               │
│Trace ambiante +2 (12 → 14, calme).                             │╰──────────────────────────────╯
│                                                                │╭ Nœuds ───────────────────────╮
│Tour 3 : Trace 14/100 (calme), cycles 3/3.                      ││ 1 Passerelle ouvert          │
│Prévision : la patrouille 1 va en Archives (3).                 ││ 2 Pare-feu   ouvert          │
│Prévision : la patrouille 1 scanne Archives (3) :               ││▶3 Archives   ici             │
│Trace +5, sauf si Cloak est actif.                              ││ 4 Coffre     ouvert          │
│Prévision : Trace ambiante +2.                                  ││ 5 Caméras    Gardien IA 0/3  │
│> breach 4 exploit                                              │╰──────────────────────────────╯
│Exploit sur Coffre (4) : progression 4/4.                       │╭ Patrouilles ─────────────────╮
│Coffre (4) est ouvert.                                          ││Patrouille 1 : 2 puis 3       │
│Trace +2 (14 → 16, calme).                                      ││                              │
│Cycles 1/3.                                                     │╰──────────────────────────────╯
│                                                                │╭ Prévision ───────────────────╮
│                                                                ││Fin de tour : Trace +7        │
│                                                                ││  Patrouille 1 : +5           │
│                                                                ││  Trace ambiante : +2         │
│                                                                ││                              │
│                                                                │╰──────────────────────────────╯
│                                                                │╭ Deck ────────────────────────╮
│                                                                ││1 Brute  2 Exploit (1)        │
│                                                                ││3 Cloak (0)                   │
╰────────────────────────────────────────────────────────────────╯╰──────────────────────────────╯
> move 4
 Tab compléter · PgUp/PgDn défiler · F2 panneau · Échap fermer · Ctrl+C quitter
```

(Capture construite à la main pour la spécification, largeurs vérifiées ; le premier snapshot `insta` de R4 la remplacera. Les formulations françaises sont indicatives : le lot de textes les arrêtera.)

### 6.2 Règles du panneau

| Zone | Contenu | Règle |
|---|---|---|
| Trace | `Trace v/100 (bande)` puis `Cycles r/m` + un point par cycle (`●` restant, `○` dépensé) | le nombre précède toujours le glyphe (GAU-1) ; en ASCII `[*--]` ; la barre graphique de la barre d'état disparaît sous 70 colonnes, jamais le libellé |
| Nœuds | une ligne par nœud : marqueur (`▶` ici, espace sinon), numéro, nom, état en mot ou défense `famille progression/force` | l'état est un **mot** (`ici`, `ouvert`) ou la défense (`Réseau 2/4`) : la couleur ne porte jamais l'état seule (COL-5) ; en ASCII le marqueur est `>` |
| Patrouilles | `Patrouille n : nœud actuel puis nœud suivant` | une patrouille neutralisée dit `(neutralisée, 2 tours)` |
| Prévision | `Fin de tour : Trace +T` puis une ligne par fait qui ajoute de la Trace | le **même** `forecast_total` que `run.forecast.total` |
| Deck | `numéro nom (charges)`, charges illimitées sans parenthèse | un programme épuisé garde sa ligne, `(0)` |

### 6.3 Budgets de largeur

Le panneau offre **30 colonnes utiles** (32 avec les bordures ; cross-check 14). Les contraintes de contenu sont donc des règles de **schéma** (R2) vérifiées au chargement, dans les deux langues, en largeur d'affichage (`unicode-width`) :

| Texte | Plafond | Ligne la plus longue prévue |
|---|---|---|
| nom de nœud | 10 colonnes | « Passerelle » (10) |
| défense `famille progression/force` | 16 colonnes | « Chiffrement 0/4 » (15), « Gardien IA 0/3 » (14) |
| ligne de nœud entière | marqueur + numéro + espace + nom (10) + espace + état (≤ 16) = 30 | « ▶3 Archives   ici », « 5 Caméras    Gardien IA 0/3 » (30) |
| nom de programme (panneau) | 8 colonnes | « Exploit » (7) |
| `Patrouille n : a puis b` | 30 colonnes | « Patrouille 1 : 2 puis 3 » (23) |

Un texte de panneau qui dépasserait est tronqué **dans le panneau seulement** par une ellipse (`...` en ASCII, jamais le numéro ni l'état) ; sa version complète reste dans le terminal et dans `map`. La tronque est une sécurité, pas un budget : le test de contenu échoue avant.

### 6.4 Paliers de taille

- **Complet (≥ 100×28)** : § 6.1.
- **Compact (≥ 64×20)** : pas de panneau permanent ; la barre d'état montre `Trace v/100 (bande) · Cycles r/m · Tour n` ; `F2` (ou `panel`) ouvre le panneau en tiroir qui recouvre le terminal et dont `Échap` sort sans perdre la saisie.
- **Trop petit** : l'écran « trop petit » existant ; l'état de l'intrusion est intact et `plain` la continue.

## 7. Importance et rôle de chaque fait

| Fait | Rôle | Importance | Brief | Normal | Full |
|---|---|---|---|---|---|
| résultat d'une action | `System` | Essential | oui | oui | oui |
| nœud ouvert, position | `System` | Essential | oui | oui | oui |
| variation de Trace | `System` (`Changed`) | Essential | oui | oui | oui |
| changement de bande de Trace | `Alert(Notice\|Warning\|Danger)` | Essential (critique) | oui | oui | oui |
| projection « Fin de tour maintenant » | `System` | Essential | oui | oui | oui |
| reste de cycles | `System` | Essential | oui | oui | oui |
| début de tour | `System` | Essential | oui | oui | oui |
| résumé de la prévision (`run.forecast.total`) | `System` | Essential | oui | non | non |
| prévision détaillée | `System` | Normal | non | oui | oui |
| mouvement et scans de fin de tour | `System` | Normal | non | oui | oui |
| scan qui coûte de la Trace | `Alert(Warning)` | Essential (critique) | oui | oui | oui |
| butin ramassé | `Reward` | Essential (critique) | oui | oui | oui |
| grillé | `Alert(Danger)` | Essential (critique) | oui | oui | oui |
| variante de texte d'ambiance (ICE qui réagit, cris d'alarme) | `Narration` | Flavor | non | non | oui |

Au `brief`, un scan dont la Trace est annulée ne s'écrit pas (la projection a déjà dit +2) ; un scan qui coûte de la Trace s'écrit toujours, parce qu'il change le jeu. Le mode lecteur d'écran n'est jamais en dessous de `normal` (VRB-3).

## 8. Familles de clés de texte

| Famille | Contenu | Variantes `@sr` |
|---|---|---|
| `run.turn.*` | début de tour, rapport | oui (nombres en mots) |
| `run.forecast.*` | `patrol_moves`, `scan`, `scan_blocked`, `ambient`, `total`, `next_turn` | oui |
| `run.use.*`, `run.breach.*`, `run.move`, `run.loot.*`, `run.probe.*` | résultats d'action, une clé par programme pour les programmes qui ont une phrase propre (Cloak, Virus, Backdoor) | quand une parenthèse ou un symbole est employé |
| `run.end.*` | `patrol_moves`, `scan`, `scan_blocked`, `ambient` | oui |
| `run.node.*`, `run.map.*` | états de nœud, colonnes de `map` | oui |
| `run.result.*` | rapport, grillé, abandon, automatique | oui |
| `run.undo.*` | tour rendu, plus d'annulation | non |
| `site.<id>.name`, `site.<id>.node.<n>.name`, `site.<id>.briefing`, `program.<id>.*` | contenu du site, écrit en données | selon le contenu |

Budget d'écriture de l'engine d'intrusion lui-même (sans le contenu des sites) : environ **90 clés par langue**, dont 25 avec variante `@sr`.

## 9. Fin d'intrusion, annulation, reprise

- **Rapport** (`jackout` ou fin automatique) : un `Screen` de 4 lignes (tours, Trace finale, butin, variation de Notoriété) et le retour au hub ; les crédits et documents sont acquis, la mission avance par `Fact`.
- **Grillé** : `Alert(Danger)` « Trace 100 : intrusion grillée. », puis le rapport (butin non ramassé perdu, Notoriété +, consommables du run perdus selon la difficulté), puis le retour au hub avec la ligne « Relance : `hack <site>`. ». En Hardcore, la partie est perdue (R-9) et revient à la dernière sauvegarde du hub.
- **Annulation** (`undo`) : rend l'état du début de l'action précédente **dans le même tour** ou le dernier tour, selon la règle retenue (§ 11, S-5) ; la ligne « Annulation : retour au tour 2, Trace 7/100 (calme). Annulations restantes : 2. » est suivie du début de tour et de la prévision, pour qu'un lecteur d'écran sache où il en est. La pile d'annulation n'est **pas** sauvegardée (cross-check 8) ; le compteur restant l'est.
- **Reprise** (`Game::resume`) : après chargement, le moteur rejoue `run.turn.start`, la prévision et l'invite, jamais l'introduction du site.
- **Sauvegarde** : une autosave à la fin de chaque tour, jamais au milieu d'une action (commands.md § 5) ; le chargement d'une sauvegarde faite en intrusion reprend donc au début de ce tour.

## 10. Exigences testables

| ID | Exigence | Test |
|---|---|---|
| RUN-1 | Tout champ de `RunView` a son équivalent texte (carte, état, défense, patrouilles, deck, prévision, cycles, annulations restantes) : un test produit les deux depuis le même état et vérifie qu'aucun chiffre de la vue n'est absent du texte de `map` + `status` | U |
| RUN-2 | **Prévision = résolution** : pour chaque état d'un bot sur 300 graines, `forecast()` égale les faits de `end_turn()` (mêmes patrouilles, mêmes nœuds, mêmes montants, même total) | U + proptest |
| RUN-3 | Ordre des lignes (§ 3) fixe : un snapshot `insta` par action de la table de commandes d'intrusion, en anglais et en français | S |
| RUN-4 | La projection n'est écrite que si la variation annoncée change ; si elle est écrite, son chiffre égale la prévision recalculée | U |
| RUN-5 | Aucune ligne de l'intrusion n'emploie un symbole de `SPOKEN_BADLY` sans variante `@sr` (`text::check` déjà en place) ; aucun glyphe de dessin n'est nécessaire à la compréhension | U |
| RUN-6 | `map` linéarisé (`--screen-reader`) : chaque ligne nomme ses colonnes ; le nombre de lignes = nœuds + 1 (patrouilles) | U |
| RUN-7 | Panneau 30 colonnes : pour chaque site du contenu et chaque langue, toutes les lignes du § 6.2 tiennent (largeur d'affichage) ; test de contenu au chargement | U |
| RUN-8 | Rendu de la TUI aux paliers 100×28, 64×20 et sous le seuil, FR et EN, `default`/`mono`/ASCII : aucune ligne ne déborde, aucune panique (I16) ; snapshots `TestBackend` | S |
| RUN-9 | En `mono`, aucun état n'est porté par la seule couleur ; en ASCII, tout octet < 0x80 (COL-3, ASC-1) | S |
| RUN-10 | Terminaison : avec Trace ambiante > 0, toute intrusion se termine en au plus T tours (I17) ; `neon-sim` mesure le T réel de chaque site | proptest |
| RUN-11 | `undo` puis la même action redonne le même état (hachage) (I18) | proptest |
| RUN-12 | Reprise : sauvegarder à la fin du tour k puis charger rejoue exactement `run.turn.start` et la prévision du tour k+1 | U |
| RUN-13 | Le transcript de la TUI est identique à celui du plain pour la même suite de commandes (EXP-2), intrusion comprise | U |
| RUN-14 | Un texte de narration (variante d'ambiance) n'est jamais nécessaire : le jeu se joue au `brief` sans en perdre un chiffre | U |

## 11. Décisions proposées

Pour validation du propriétaire (S-4 et S-5 de [`DECISIONS.md`](../design/DECISIONS.md) § 5) :

- **S-4** : la prévision est détaillée au début du tour et résumée par la **projection** après les actions qui la changent (pas à chaque action) ; elle n'annonce jamais l'aléa ; elle ne mentionne Cloak que s'il peut être utilisé.
- **S-5** : `undo` annule **le tour entier en cours** (retour à son début) et non une action isolée : c'est le coût de calcul le plus bas et le comportement le plus lisible à voix haute. Variante possible : annulation d'action, avec la même pile.

## 12. Limites et ce qui reste à écrire

- Ce document ne fixe ni le style, ni la longueur des textes d'ambiance : lot de textes L2.
- Les maquettes sont à la main ; le premier snapshot automatique est un livrable de R4.
- Le brouillard de guerre (nœuds inconnus, `probe` obligatoire) est décrit mais pas exercé ici ; le second spike du solveur (lot P1.2, `spikes/solver`) mesure le coût de la connaissance partielle.
- L'écran de fin de campagne, le titre et les options sont hors de ce document (cross-check, trous).
