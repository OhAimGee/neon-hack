# TUI, frontend plain et accessibilité : conception et spike vérifié

> Livrable de la tâche C (réécriture en Rust, conception libre). Dépôt : branche `claude/relaxed-lamport-87hbbr`, HEAD `653bc46`. Rien n'a été modifié dans le dépôt en dehors de ce fichier.
> Le spike (workspace Cargo complet, 56 tests) vit hors dépôt, dans le scratchpad de la session : `spike-tui/`. Les extraits de code cités ici en sont tirés et ont tous été compilés. Les identifiants et le code restent en anglais, le reste en français.

**Légende des preuves** — `[E]` exécuté (test ou mesure du spike, rustc 1.97.0, Linux) ; `[C]` lu dans le code du dépôt C ou dans le source d'une crate (`~/.cargo/registry`) sans l'exécuter ; `[D]` connaissance de la documentation d'une plateforme ou d'une norme, non vérifiée ici ; `[H]` hypothèse ou conception non exercée. Ce qui n'a **pas** pu être vérifié est regroupé en 7.5.

---

## 0. Résumé

1. **Un moteur pur, deux vues.** Le moteur ne fait aucune E/S, n'a ni horloge ni langue. Il reçoit un `Input` et renvoie un `Step { events, prompt }` ; il expose en plus `view()` (instantané pour la barre d'état et le panneau) et `complete(before)` (candidats TAB). Les événements portent un **rôle sémantique** (narration, dialogue(speaker), système, alerte, gain, erreur, décor, écran structuré) et un **texte sous forme de clé + arguments typés**. Une seule fonction, `render_event`, met un événement en lignes de texte ; elle sert au frontend plain, au journal de la TUI, aux transcripts et aux tests : *la TUI ne montre rien que le plain ne dise* (§ 2).
2. **i18n résolue au rendu, pas dans le moteur** (recommandation, § 2.3) : le journal de la TUI garde des événements, un changement de langue en partie réécrit l'historique, les tests du moteur ne dépendent d'aucune phrase, un lecteur d'écran obtient une variante sans symboles (`clé@sr`) [E].
3. **Piles retenues** (versions exactes trouvées sur crates.io le 2026-10-07 et compilées ensemble [E]) : `ratatui 0.30.2` (et son ré-export `ratatui::crossterm`, donc `crossterm 0.29.0` sans dépendance directe), `unicode-width 0.2.2`, `unicode-segmentation 1.13.3`, `clap 4.6.7`, `anstream 1.0.0` (sortie couleur du plain), `signal-hook 0.4.5` (Unix) ; en test `insta 1.49.0`, `portable-pty 0.9.0`, `vt100 0.16.2`. MSRV imposé par la plus exigeante : **1.88** (ratatui 0.30.2).
4. **TUI à trois paliers** : ≥ 100×28, panneau latéral permanent ; ≥ 64×20, journal pleine largeur et panneau en tiroir (F2) ; en dessous, repli sur le plain au démarrage [E] ou écran « trop petit » en cours de partie [E]. `Ctrl+P` bascule vers le plain **sans perdre la partie** [E] : c'est la preuve concrète que le frontend est jetable.
5. **Saisie : widget maison** (≈ 230 lignes de code, 8 tests, [E]) plutôt qu'une crate. Aucune des crates évaluées ne fournit historique ni TAB ; `tui-textarea 0.7.0` **ne compile plus** avec ratatui 0.30 [E] ; `ratatui-textarea 0.9.3` découpe un drapeau en deux [E] ; `tui-input 0.15.5` est correct sur les graphèmes [E] et reste le plan B.
6. **Accessibilité** : 60 exigences numérotées et testables (§ 5), dont 40 déjà vérifiées automatiquement dans le spike (certaines sur son seul périmètre). Les points durs (`NO_COLOR` avec précédence dans le plain **et** la TUI, aucun octet ESC en mode lecteur d'écran, `--ascii` strict 7 bits, jauges toujours chiffrées et nommées, aucune sortie spontanée en veille, panique/SIGTERM/Ctrl+Z qui rendent le terminal propre) sont tous couverts par des tests automatiques (pseudo-terminal ou `TestBackend`).
7. **Tests** : `TestBackend` + `insta` pour la TUI, golden transcripts pour le plain, **14 tests pty** (`portable-pty` + `vt100`) qui remplacent `tests/e2e/pty_hud.py` (751 lignes, mini-émulateur maison). Déterminisme par graine, environnement injecté, aucune horloge (§ 6).
8. **Pièges rencontrés** (détail en 7.4) : `ratatui::restore()` ne rend ni la souris, ni le collage bracketé, ni le curseur, et avec `panic = "abort"` seul le hook de panique s'exécute (test de mutation [E]) ; `Terminal::clear()` interroge la position du curseur et échoue sans réponse du terminal ; `#[arg(env)]` de clap sur un booléen **refuse `NEON_SCREEN_READER=1`** et fait planter le jeu au lancement ; `Paragraph::line_count` est instable ; `SIGTSTP` est ignoré dans un groupe de processus orphelin.
9. **8 décisions à valider** (§ 8). **Checklist d'acceptation** au § 9.

---

## 1. L'interface actuelle en C et ses limites (lu dans le code)

`src/ui/*` = 1 754 lignes, plus `shop_view.c` (458) et `complete.c` (108) ; tests : `tests/unit/test_{hud,lineedit,term,shop_view,complete}.c` (1 790 lignes) et `tests/e2e/pty_hud.py` (751 lignes). `[C]` partout dans ce tableau sauf mention.

| Élément | Où | Limite constatée | Conséquence pour le nouveau design |
|---|---|---|---|
| Barres haut/bas par **région de défilement** ANSI | `hud.c:175-178`, ARCHITECTURE « Limite connue » | Aucun panneau à droite possible ; l'historique appartient au terminal, remonter la molette emporte les barres | La TUI **possède l'historique** (liste d'événements) et dessine tout l'écran |
| Redessin | `nh_hud_refresh` avant chaque invite (ARCHITECTURE l.358-359) | Rien pendant l'attente d'une saisie, un redimensionnement n'est vu qu'à la validation | Boucle pilotée par les événements clavier/redimensionnement (§ 4.8) |
| Saisie | `lineedit.c` (866 l.) = cœur pur + colle termios | Windows : repli sans édition ; historique 32 lignes de session ; ligne 256 octets ; menu, prologue et boutique **non éditables** | Un seul éditeur pour toutes les invites ; Windows de plein droit (crossterm) |
| Largeur d'affichage | `term.c:81-99` (`k_zero`, `k_wide` à la main) | Pas de graphème : `U+FE00-FE0F` compté 0, `U+2764` seul 1 ; `U+2764 U+FE0F` (cœur à présentation emoji) = **1** colonne en C contre **2** avec `unicode-width 0.2.2` [E] | Tables Unicode d'une crate maintenue ; `--ascii` pour les terminaux à largeur ambiguë |
| Couleurs | `NhColor` (10 valeurs), `legacy_colors.h` dans les modules non portés (ARCHITECTURE « Écarts assumés ») | `--no-color` incomplet ; une couleur par rôle, aucune palette alternative | Styles **sémantiques** et palettes interchangeables (§ 5.1) |
| Jauge et décor | `nh_gauge` (`term.c:161`, `█░` sans repli), bandeau de boutique en block-art (`shop_view.c:37-40`), filets `─` | Aucun repli ASCII, aucun texte de remplacement | `Event::Decor { art, alt }`, jauge ASCII `[###---]` |
| Temps imposé | `nh_typewriter` (`term.c:312-325`) : `fwrite` + `sleep` par caractère, **non interruptible** ; pauses dans les handlers | 790 s d'attente cumulée pour un joueur complétiste (audit § 2.4) | Aucune attente dans le moteur ; la cadence est un réglage de présentation, interruptible |
| Vitrine de la boutique | `shop_view.c` : composition **pure** (`nh_shop_compose`), 3 mises en page selon un budget de lignes, 2 colonnes dès 71 colonnes utiles, état toujours dit en mots | Mise en page calculée à la main | Garder : composition pure, « l'écran n'annonce jamais ce que l'achat refuserait », état en mot **et** en couleur. `Table` + `Layout` de ratatui remplacent la mise en page manuelle |
| Complétion | `complete.c` (108 l.) | — (bon design) | Garder : `Game::complete(before)` ne propose que ce que le joueur peut utiliser maintenant |
| Tests d'interface | `pty_hud.py` : émulateur d'écran écrit pour ce jeu | Ne sait faire que ce que le jeu utilise ; Python en plus de Rust | `vt100` (vrai émulateur : écran alternatif, souris, collage, attributs) dans `cargo test` |

À conserver tel quel : le HUD n'est jamais actif sur un tube ; `NO_COLOR` plus fort que le fichier de réglages (seul `--color` le contredit) ; la complétion déduite de l'état ; la composition pure testable sans terminal.

---

## 2. Contrat moteur ↔ frontends

### 2.1 Principes

- **P-1** Le moteur est une fonction `(état, Input) → (état, Step)`. Pas d'E/S, pas d'horloge (`Instant`/`SystemTime` interdits dans `neon-engine`, vérifiable par `clippy::disallowed_methods`), pas de langue, pas de largeur d'écran.
- **P-2** Un frontend est une **vue jetable** sur le moteur : on peut en changer en cours de partie (`Ctrl+P`, § 4.4) sans rien perdre.
- **P-3** *Même action, même commande.* Rien n'est faisable dans la TUI qui ne le soit par une ligne de texte ; les raccourcis et les modales ne font que soumettre le même `Input`.
- **P-4** *Même contenu.* Ce que la TUI affiche en widgets, le plain l'écrit en texte, et inversement : les deux consomment les mêmes `Event`.

```
        Input (Line | Choice | Confirm | Continue)
 frontend ───────────────────────────────────────▶ moteur (pur)
    ▲                                                 │
    └──────── Step { events: Vec<Event>, prompt } ◀───┘
 + moteur.view()                  -> View          (lecture, pour barre d'état / panneau)
 + moteur.complete(avant_curseur) -> Completions   (TAB, déduit de l'état du jeu)
```

### 2.2 Types (extraits du spike, compilés [E])

```rust
// neon-engine : texte = clé + arguments typés (jamais une phrase déjà construite)
pub struct Text { pub key: &'static str, pub args: Vec<(&'static str, Arg)> }
pub enum Arg { Int(i64), Str(String), Term(&'static str), Text(Box<Text>) }

pub enum Role { Narration, Dialogue { speaker: &'static str }, System, Alert(Severity), Reward, Error }
pub enum Importance { Essential, Normal, Flavor }          // filtre de verbosité (§ 5.7)

pub enum Event {
    Message { role: Role, importance: Importance, text: Text },
    Decor { art: &'static [&'static str], alt: Text },     // dessin pur : retiré en mode lecteur d'écran
    Screen(Screen),                                        // Shop(Table) | Map(Table) | … : modale (TUI), liste numérotée (plain)
    Trace { from: i32, to: i32 },                          // valeur suivie ; à généraliser en Changed { gauge, from, to }
    Break, Quit,
}
pub struct Table { pub title: Text, pub columns: Vec<Text>, pub rows: Vec<Vec<Text>> }

pub enum Prompt {
    Command,                                               // invite principale (TAB via complete())
    Choice(Choice),                                        // menu : réponse par numéro OU par nom
    Text { label: Text, max_chars: usize, default: Option<String> },
    Confirm { question: Text, default: bool },
    Continue,                                              // « Entrée pour continuer » : remplace chaque pause chronométrée
}
pub struct Choice { pub title: Text, pub options: Vec<ChoiceOption>, pub cancel: Option<Text> }
pub struct ChoiceOption { pub label: Text, pub available: Result<(), Text> } // indisponible = raison dite en mots
pub enum Input { Line(String), Choice(usize), Confirm(bool), Continue }
pub struct Step { pub events: Vec<Event>, pub prompt: Prompt }
```

Dans le spike, `Game::step(&str)` renvoie encore `Vec<Event>` : `Prompt`, `Input` et `Step` compilent mais ne sont pas câblés (les menus ne sont pas exercés, `[H]`).

**Rendu par rôle** (`render_event` pour le plain ; la TUI utilise le même texte et ajoute un style) :

| Événement | Plain | TUI | Mode lecteur d'écran |
|---|---|---|---|
| `Narration`, `System` | le texte | style narration/système | idem |
| `Dialogue { speaker }` | `ECHO-7 » texte` (`--ascii` : `ECHO-7: texte`) | nom en gras + couleur | `ECHO-7: texte` |
| `Alert(sev)` | `[ALERTE] texte` | avertissement ; danger en **vidéo inverse** | idem |
| `Reward`, `Error` | `[Gain] texte`, `[Erreur] texte` | couleur ; erreur soulignée | idem |
| `Decor { art, alt }` | l'art (`--ascii` : `alt`) | style titre | omis (`alt` seulement en `full`) |
| `Screen(table)` | titre puis `[n] a - b - c` | modale `Table` **et** lignes dans le journal | `n. Colonne : valeur ; Colonne : valeur` |
| `Trace { from, to }` | `Trace +10 (4 → 14, calme).` | jauge du panneau + la même ligne au journal | `Trace en hausse de 10, de 4 à 14, niveau calme.` |
| `Break` | ligne vide | séparateur | ligne vide (omise en `brief`) |

Invariants prouvés [E] : une session complète ne produit jamais `<missing…>` dans aucune langue ni aucun mode ; tout `Error`, `Alert` et `Reward` est `Essential`, donc aucun niveau de verbosité ne peut le masquer.

### 2.3 i18n : au rendu (recommandation)

Le moteur n'émet que des clés et des arguments ; `render(&Text, Lang, Mode) -> String` est une fonction pure, dans `neon-engine::text` (catalogue embarqué), appelée par les frontends, les transcripts et `neon-sim`.

| Pourquoi au rendu | Preuve |
|---|---|
| Les tests du moteur affirment sur des clés, pas sur des phrases : ils ne bougent pas quand on retouche un texte | `same_seed_same_events` [E] |
| Sauvegarde et rejeu indépendants de la langue ; le même rejeu donne un transcript FR ou EN | [H] |
| Le journal de la TUI conserve des `Event` : **changer de langue en partie réécrit tout l'historique** | conception, `LogEntry::Event` [E] |
| Le frontend choisit la variante sans que le moteur le sache : `clé@sr` (lecteur d'écran), `--ascii`, largeur | `render_mode` [E], golden EN/SR [E] |
| La jauge a besoin des **nombres** (`Arg::Int`), pas d'une phrase | panneau + barre d'état [E] |
| Le contenu narratif sera en fichiers de données : le moteur ne peut pas embarquer de phrases | audit P4 |

Coûts et parades : plus aucune concaténation de phrase dans le moteur. Les pluriels et accords passent par le catalogue (`{n|un nœud|{n} nœuds}`, `{n||s}` ; forme singulière pour 0 en français) [E] ; les noms d'objets sont des `Arg::Text` imbriqués [E] ; une clé par variante de phrase. Garde-fous testés [E] : parité FR/EN **des marqueurs** (un `{trace}` absent d'une langue échoue), un `@sr` a les mêmes marqueurs que sa clé de base, aucune clé manquante dans une session. À ajouter côté données : toute clé citée par un fichier de contenu existe (test statique au chargement).

L'alternative (le moteur résout la langue) est écartée : il lui faudrait connaître langue, mode lecteur d'écran et jeu de caractères, le journal ne pourrait plus changer de langue, et la jauge devrait relire des chiffres dans une phrase.

### 2.4 `View`, écrans, menus

- `View` = lecture seule, calculée à la demande : joueur, crédits, Trace et sa bande, quêtes actives, nœuds connus, inventaire. La TUI la lit à chaque image ; le plain n'en a besoin que pour `status`.
- Un `Screen` (boutique, contacts, courrier, journal, carte) est une `Table` structurée. Le plain l'imprime en liste numérotée ; la TUI l'ouvre en modale **non capturante** (§ 4.3). Les actions (`buy 3`) sont des lignes de commande ordinaires.
- Un `Prompt::Choice` se répond par **numéro ou par nom** (comme `contact <n|nom>` aujourd'hui) ; une option indisponible dit sa raison en mots (reprise de la règle de la vitrine C).

### 2.5 Cadence

Le moteur ne contient aucune durée. La « machine à écrire » est une présentation : état d'avancement d'un événement dans le journal, avancé par `event::poll(timeout)` uniquement pendant qu'elle court, interrompue par n'importe quelle touche (la touche est ensuite traitée normalement) et plafonnée à 2 s par événement quelle que soit sa longueur `[H]`. Réglages : `off` | `fast` (≈ 400 car/s) | `normal` (≈ 120) | `slow` (≈ 40). Défauts : `off` en plain, en mode lecteur d'écran et avec `--reduce-motion` ; `fast` en TUI (cohérent avec la décision 9 de l'audit).

### 2.6 Découpage

| Où | Quoi |
|---|---|
| `neon-engine` | `text` (catalogue, `render`, modes), `Event`/`Prompt`/`Step`/`View`, `complete`. Aucune dépendance terminal |
| `neon-cli::opts` | `EnvSnapshot` (l'environnement lu **une seule fois**, injectable), précédence des réglages |
| `neon-cli::render` | `render_event` : événement → lignes (commun au plain, au journal TUI, aux transcripts) |
| `neon-cli::plain` | boucle ligne à ligne |
| `neon-cli::tui::{app, layout, panel, modal, input, palette, glyphs, term}` | la TUI |

Taille du spike (tests unitaires inclus) : ≈ 2 300 lignes de frontend, ce qui recoupe l'estimation de l'audit (plain 0,7-1 k, TUI 2-3 k).

---

## 3. Frontend plain

Ligne par ligne, linéaire, déterministe, sans adressage du curseur. C'est le frontend **de référence** : celui des tubes, des tests, des lecteurs d'écran, de `--transcript`.

- **Boucle** : afficher l'invite *avant* de lire (sur un TTY, l'utilisateur doit la voir pendant qu'il tape ; le spike l'imprimait après et un test pty l'a révélé [E]), lire une ligne, répondre, **vider la sortie à chaque commande**. Sur un tube, la ligne lue est répétée en sortie (`> scan`) pour que le transcript soit lisible ; sur un TTY, le terminal fait l'écho. Fin d'entrée (`EOF`) : on termine proprement, jamais de boucle.
- **Sur un TTY : mode « cuit »** (aucun mode brut) : l'édition de ligne est celle du terminal ou du système, donc celle que les lecteurs d'écran connaissent. Pas de TAB en plain : les menus se répondent par numéro ou nom, `help` liste les commandes (décision 2). Pas de `rustyline` (il redessine la ligne avec des séquences de curseur `[H]`).
- **Aucun retour à la ligne applicatif** : un paragraphe = une ligne logique. Les lecteurs d'écran et les afficheurs braille lisent ligne par ligne ; couper une phrase au milieu les fait marquer une pause. (Le C coupait à la largeur du terminal avec retrait : option `--wrap` pour les voyants, `[H]`.)
- **Couleur** : jamais sans décision explicite (§ 5.1). Sortie via `anstream::AutoStream::new(stdout, ColorChoice::{Always|Never})` : en `Never` toute séquence SGR est supprimée, sous Windows 10+ le mode VT est activé [E, `anstream_check`]. Le plain n'utilise que les 8 couleurs ANSI et le gras, jamais 256 couleurs ni RGB.
- **Déterminisme** : aucune date, aucun chemin, aucune largeur de terminal dans la sortie ; pas d'ANSI hors TTY. Même graine, mêmes octets [E].
- **Codes de sortie** : 0 fin normale, `quit` ou EOF ; 1 erreur d'E/S ; 2 usage (clap).
- **Transcript** : `--transcript FICHIER` ajoute et vide à chaque commande (un plantage ne perd rien) ; contenu = le rendu plain sans ANSI [E].
- **Windows** : la console est gérée par `std` (conversion UTF-8 ↔ UTF-16), plus de `SetConsoleOutputCP` à la main `[D]`.

---

## 4. Frontend TUI

### 4.1 Disposition

Trois paliers, calculés sur la taille **à chaque image** :

| Palier | Taille | Contenu |
|---|---|---|
| Full | ≥ 100×28 | barre d'état, journal encadré, panneau de 32 colonnes permanent, saisie, barre d'aide |
| Compact | ≥ 64×20 | idem sans encadrement du journal ; le panneau est un tiroir (`F2`, ou une commande `panel` à prévoir) |
| Trop petit | sinon | une phrase : « Terminal trop petit : 50×15. Minimum 64×20… » ; l'état de la partie est conservé |

Rendu réel du spike à 100×30 (palette `mono`, anglais ; `TestBackend`, snapshot `insta` [E]) :

```
 Case · 197 credits · ████░░░░░░ Trace 40/100 (watchful)
╭ Log ─────────────────────────────────────────────────────────────╮╭ Trace ───────────────────────╮
│█▄ █ █▀▀ █▀█ █▄ █                                                 ││████████████░░░░░░░░░░░░░░░░░░│
│█ ▀█ ██▄ █▄█ █ ▀█                                                 ││Trace 40/100 (watchful)       │
│NEON HACK · Neo-Tokyo, 2087                                       │╰──────────────────────────────╯
│ECHO-7 » Type help, Case. I'll guide you.                         │╭ Quests ──────────────────────╮
│> scan                                                            ││✓ First steps: breach a node  │
│Scan: one node revealed.                                          ││  (done)                      │
│Trace +10 (4 → 14, calm).                                         │╰──────────────────────────────╯
│> scan                                                            │╭ Network ─────────────────────╮
│Scan: 2 nodes revealed.                                           ││✓ 1. corp-gateway (breached)  │
│T╭ R4Z0R's shop ────────────────────────────────────────────────╮ ││○ 2. payroll-db (open)        │
│>│#   Item                Price              State              │ ││○ 3. archive-7 (open)         │
│[│1.  Ghost Protocol      80 ¢               available          │ ││                              │
│T│2.  Dark Web VPN        60 ¢               available          │ ││                              │
│[│3.  Quantum Chip        800 ¢              603 credits short  │ ││                              │
│>│                                                              │ ││                              │
│R╰─────────────────────────────────────────────────── Esc close ╯ ││                              │
│[1] Ghost Protocol - 80 ¢ - available                             ││                              │
│…                                                                 │╰──────────────────────────────╯
│                                                                  │╭ Inventory ───────────────────╮
│                                                                  ││(empty)                       │
╰──────────────────────────────────────────────────────────────────╯╰──────────────────────────────╯
>
Tab complete · PgUp/PgDn scroll · F2 panel · Esc close · Ctrl+C quit
```

(lignes vides du journal supprimées pour la lecture.) Toute information portée par une couleur l'est aussi par un mot : `Trace 40/100 (watchful)`, `✓ … (breached)`, `[Reward]`, `(done)`.

| Zone | Contenu et règles |
|---|---|
| Barre d'état (1 ligne) | nom · crédits · jauge + **libellé chiffré** `Trace 40/100 (vigilance)`. La barre graphique disparaît sous 70 colonnes, le libellé jamais |
| Journal | historique **tenu par l'application** : liste d'`Event`, rewrappée à la largeur courante (le spike ne wrappe que la queue visible : 110 µs par image avec 100 000 entrées au bas, **11 ms** à 20 000 lignes de profondeur, [E] release → cache des lignes par largeur/langue au-delà). Plafond conseillé : 10 000 entrées (le transcript sur disque garde tout). `PgUp/PgDn/Esc`, indicateur « ↑ historique (n lignes plus bas) » |
| Panneau | Trace (jauge **et** libellé), Quêtes (objectifs actifs), Réseau (nœuds connus, glyphe + mot d'état), Inventaire (deck, consommables). Hauteurs calculées sur le contenu **déjà wrappé** par nos soins : `Paragraph::line_count` est derrière la feature `unstable-rendered-line-info` `[C]` |
| Saisie | `> ` + éditeur (§ 4.2), défilement horizontal, curseur placé par `Frame::set_cursor_position` |
| Barre d'aide | raccourcis par ordre d'importance, tronquée par la fin |

### 4.2 Ligne de saisie : construire ou prendre une crate

| Critère | Widget maison | `tui-input 0.15.5` | `ratatui-textarea 0.9.3` | `tui-textarea 0.7.0` | `rustyline 18.0.1` / `reedline 0.52.1` |
|---|---|---|---|---|---|
| Compile avec ratatui 0.30.2 | oui | oui (dépend de `ratatui ^0.30.2`) [E] | oui (`ratatui-core ^0.1.1`) [E] | **non** : tire ratatui 0.29.0 et crossterm 0.28.1, `E0277` ×2 [E] | possèdent leur propre boucle et leur propre mode brut `[D]` : ne se branchent pas sur un `Frame` |
| Curseur par **graphème** (`e`+U+0301, drapeau) | oui [E] | oui [E] | **non** : retire une moitié de drapeau [E] | n/d | n/d |
| Historique | à écrire (20 lignes) | non | non | non | oui |
| **TAB depuis l'état du jeu** | oui (`complete()` injecté) | non | non (TAB insère 4 espaces [E]) | non | oui (trait), mais possède le terminal |
| Une seule ligne | oui | oui | non : Entrée coupe la ligne [E] | non | oui |
| Collage sans exécution | oui [E] | `[H]` | `[H]` | `[H]` | `[H]` |
| Coût | 351 lignes dont 228 de code et 8 tests | ~ 60 lignes économisées | idem | — | redessin de la ligne par séquences de curseur : à éviter avec un lecteur d'écran `[H]` |

**Recommandation : widget maison**, pur (aucun accès au terminal), testé touche par touche (le cœur C `nh_le_*` l'était octet par octet). Il fournit : curseur par graphème (`unicode-segmentation`), colonne par largeur (`unicode-width`), historique avec brouillon conservé, TAB à la bash (un seul candidat → complété avec espace ; plusieurs → plus long préfixe commun, **sans tenir compte de la casse, le mot inséré prend celle du candidat** ; sans progression, un second TAB liste), Ctrl+A/E/B/F/U/K/W/P/N, collage qui remplace les retours à la ligne par des espaces. Si l'entretien devient pesant, `tui-input` le remplace (API proche : `value()`, `cursor()`).

Touches, y compris **AltGr** : sous Windows, AltGr arrive comme `CONTROL|ALT` `[D]` ; un clavier AZERTY tape `@ # { [ ] }` avec AltGr. La règle « un caractère s'insère si les modifieurs sont vides, `SHIFT`, ou exactement `CONTROL|ALT` » est testée [E] ; sans elle, un joueur francophone sous Windows ne pourrait pas écrire `@`.

### 4.3 Écrans modaux

Une modale est une **vue épinglée**, jamais un mode : la saisie reste active, `Échap` la ferme, elle se met à jour après chaque commande, elle est centrée sur la zone du journal (le panneau reste visible : on garde l'œil sur la Trace en achetant).

| Écran | Ouvert par | Contenu (même `Table` que le plain) | Dans la modale | Équivalent plain |
|---|---|---|---|---|
| Boutique | `shop` | objets, prix, **état en mots** (« il manque 603 crédits ») | `buy <n>` | liste `[n] objet - prix - état` |
| Contacts | `contacts` | contacts débloqués, relation | `contact <n\|nom>` | liste numérotée |
| Courrier | `messages` | boîte, NOUVEAU marqué en mot | `read <n>` | liste ; `read` imprime le message |
| Journal de quêtes | `quests` | quêtes, objectifs, avancement | — | liste |
| Carte du réseau | `map` | nœuds, liens, état | `hack <n\|nom>` | liste de liens |
| Inventaire / deck | `deck` | programmes, charges | `equip <n>` | liste |

Dans le spike : `Shop` et `Map` [E] ; les autres sont la même mécanique `[H]`. Dans le palier Compact, une modale plus haute que la zone défile avec `PgUp/PgDn` `[H]`.

### 4.4 Redimensionnement, taille minimale, repli

- `Event::Resize` (émis par SIGWINCH sous Unix, nativement par crossterm) ; la boucle redessine ; ratatui détecte le changement de taille au `draw`. Test pty : 100×30 → 80×24 (panneau parti) → 50×15 (« trop petit ») → 100×30 (panneau revenu) [E].
- **Démarrage** sous 64×20 : repli automatique sur le plain, une ligne sur `stderr` annonce pourquoi, aucun écran alternatif [E].
- **En cours de partie** : pas de bascule automatique (surprenante) ; l'écran « trop petit » s'affiche, la partie est intacte, `Ctrl+P` passe au plain. Ce passage rend le moteur à `plain::run_session(boot = false)` : l'état est conservé (même Trace avant et après) [E] ; le transcript continue le même fichier en ajout (code, non testé).
- Un terminal à largeur ambiguë « large » (locales CJK) compte `█ ─ → · … × ○ •` sur 2 colonnes (`width_cjk`) [E] : les cadres se décalent. `--ascii` est le remède (§ 5.8).

### 4.5 Cycle de vie du terminal : restauration garantie

Le point où l'on se trompe le plus. Ce que `ratatui::try_init()` fait **et ne fait pas** `[C]` : il active le mode brut, entre dans l'écran alternatif et installe un hook de panique qui appelle `ratatui::restore()` ; `restore()` ne fait que quitter le mode brut et l'écran alternatif. Souris, collage bracketé et curseur restent à notre charge.

```rust
pub fn restore_all() {                       // idempotent : Drop, hook de panique, thread de signaux
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste, Show);
    ratatui::restore();                      // mode brut + écran alternatif
}
pub struct TerminalGuard;                    // Drop = restore_all()

pub fn enter(mouse: bool) -> io::Result<(DefaultTerminal, TerminalGuard)> {
    let terminal = ratatui::try_init()?;     // + hook de ratatui
    let guard = TerminalGuard;
    let previous = std::panic::take_hook();  // notre hook devant celui de ratatui
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste, Show);
        previous(info);
    }));
    execute!(stdout(), EnableBracketedPaste)?;
    if mouse { execute!(stdout(), EnableMouseCapture)?; }
    #[cfg(unix)] install_signal_restore();   // SIGTERM/HUP/QUIT/INT venus de l'extérieur
    Ok((terminal, guard))
}
```

Vérifié en pseudo-terminal, en lisant l'état de l'émulateur (`alternate_screen()`, `hide_cursor()`, `bracketed_paste()`, `mouse_protocol_mode()`) [E] :

| Sortie | Résultat |
|---|---|
| Ctrl+C (en mode brut, c'est une **touche**) ou `quit` | écran, curseur, collage, souris restaurés ; code 0 |
| Panique (touche F12 de test) | idem, et le message de panique est lisible sur l'écran normal ; code ≠ 0 |
| `kill -TERM` de l'extérieur | idem avant de mourir (un thread `signal-hook` restaure puis `emulate_default_handler`) ; statut de sortie non nul |
| Ctrl+Z | rend le terminal, **s'arrête** (état `T`), puis `kill -CONT` : écran alternatif ré-entré et image redessinée |

Deux preuves par **mutation** : (1) en retirant la restauration de notre hook, le test de panique **passe encore** (le `Drop` du garde s'exécute pendant le déroulement) ; (2) avec `panic = "abort"`, il **échoue** (« panic hook must disable bracketed paste ») : seul le hook s'exécute. Le garde (`Drop`) couvre le déroulement normal, le hook couvre `abort` : on garde les deux.

Autres constats [E] : `Terminal::clear()` (ratatui 0.30.2) lit d'abord la position du curseur (`ESC[6n`) et échoue avec « The cursor position could not be read within a normal duration » quand rien ne répond (émulateur de test, multiplexeur, console série) → après une reprise on utilise `terminal.resize(Rect)`, qui efface sans cette requête ; `SIGTSTP` est **ignoré** dans un groupe de processus orphelin (c'est le cas sous un harnais de test) : la suspension utilise `SIGSTOP`.

### 4.6 Windows, macOS, Linux

| Sujet | Comportement | Preuve |
|---|---|---|
| Compilation | `cargo check --workspace --all-targets` passe pour `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `aarch64-apple-darwin` (cibles ajoutées avec `rustup target add`) | [E] compilation seule |
| Touches | crossterm émet `Press` **et** `Release` sous Windows : on ignore `Release` (sinon chaque touche compte double) | `[C]` `parse.rs` émet `Release` ; filtre écrit et compilé, pas exécuté sous Windows |
| AltGr | `CONTROL\|ALT` sous Windows | règle testée [E] ; comportement réel `[D]` |
| Terminal | Windows Terminal et conhost récent (Windows 10+, VT) ; pas de prise en charge de la console Windows 7/8 | `[D]` |
| Ctrl+C / Ctrl+Z | Ctrl+C = touche en mode brut ; Ctrl+Z sans effet (pas de contrôle des tâches) ; `Ctrl+Break` tue sans restauration (`SetConsoleCtrlHandler`/`ctrlc`) | `[D]`, `[H]` |
| Tests pty | `portable-pty` gère ConPTY, mais la sortie ConPTY est réécrite (écran redessiné, séquences ajoutées) et ne se prête pas à une assertion fiable avec `vt100` : tests pty `#![cfg(unix)]` ; Windows = `TestBackend` + un test plain par tube | décision de conception `[H]` |
| macOS | mêmes tests pty attendus | `[D]`, non exécuté |

### 4.7 Souris, collage, focus

- **Souris : désactivée par défaut** (`--mouse` ou réglage). La capturer empêche la sélection/copie native du terminal (il faut Maj+glisser). Quand elle est active, seule la molette fait défiler le journal ; **rien** n'est faisable uniquement à la souris [E : molette → « ↑ historique », `Échap` revient en bas].
- **Collage bracketé : toujours activé.** Un collage multi-lignes ne s'exécute jamais : les retours à la ligne deviennent des espaces, l'utilisateur valide lui-même [E].
- Pas d'événements de focus.

### 4.8 Modèle de redessin

Le moteur n'a pas de minuterie, donc **la TUI est purement pilotée par les événements** : `event::read()` bloquant, un `draw` par événement. En veille, elle n'écrit rien du tout [E : 1,5 s de silence, zéro octet]. Pas de curseur clignotant dessiné par l'application, pas de spinner. `poll(timeout)` n'est utilisé que pendant qu'une machine à écrire court (§ 2.5).

---

## 5. Spécification d'accessibilité

Chaque ligne est une exigence **testable**. Colonne « Test » : `U` unitaire, `S` snapshot ou balayage de `Buffer` (`TestBackend`), `P` pseudo-terminal, `L` lint ou grep en CI, `M` manuel. Colonne « Spike » : ✔ = déjà vérifié par exécution, ○ = à écrire.

### 5.1 Couleur

| ID | Exigence | Test | Spike |
|---|---|---|---|
| COL-1 | Précédence : `--no-color`/`--color` explicite > `NO_COLOR` non vide > réglage enregistré > autodétection (TTY et `TERM` ≠ `dumb`). `NO_COLOR=` (vide) est ignoré (no-color.org) `[D]` | U | ✔ |
| COL-2 | `NO_COLOR` et `--no-color` valent pour le plain **et** la TUI ; `NO_COLOR=1` met la sortie en palette `mono` | P | ✔ |
| COL-3 | En `mono`, la TUI n'emploie que des **attributs** (gras, inverse, souligné, italique) : aucune cellule du `Buffer` avec `fg`/`bg` ≠ `Reset`, à tous les paliers, modale comprise | S | ✔ |
| COL-4 | Jamais de clignotement : aucun `SLOW_BLINK`/`RAPID_BLINK` dans aucun style ni aucune cellule | U + S | ✔ |
| COL-5 | Aucune information portée par la couleur seule : chaque état a un mot, chaque niveau de danger un attribut (vidéo inverse) en plus | S | ✔ (jauge, quêtes, réseau, [Alerte]) |
| COL-6 | Palettes : `default` (couleurs ANSI nommées, **transparente**, suit le thème), `high-contrast`, `cvd`, `mono`. Les deux palettes **opaques** peignent chaque cellule de leur fond | S | ✔ |
| COL-7 | Texte des palettes opaques ≥ **4,5:1** (WCAG AA) contre leur fond, calculé par un test ; éléments graphiques ≥ 3:1. `high-contrast` = `#FFFFFF` sur `#000000` (21:1) en truecolor ; sur 16 couleurs, « blanc » et « noir » sont ceux du thème : rapport non certifiable | U | ✔ (`cvd` ≥ 4,9:1 ; `high-contrast` truecolor 21:1) |
| COL-8 | `Color::Rgb` seulement si `COLORTERM` ∈ {`truecolor`, `24bit`}, sinon 16 couleurs | U | ○ |
| COL-9 | Plain : SGR 8 couleurs + gras seulement ; le transcript est identique avec ou sans couleur | U | ✔ |

Contraste mesuré (script `contrast.py`, formule WCAG 2.x [E]) :

| Couleur Okabe-Ito | sur `#101010` (fond opaque de `cvd`) | sur `#000000` | sur `#FFFFFF` |
|---|---|---|---|
| bleu ciel `#56B4E9` | 8,25 | 9,10 | 2,31 |
| orange `#E69F00` | 8,45 | 9,32 | 2,25 |
| jaune `#F0E442` | 14,39 | 15,88 | **1,32** |
| vermillon `#D55E00` | 4,92 | 5,43 | 3,87 |
| pourpre `#CC79A7` | 6,22 | 6,86 | 3,06 |
| vert bleuté `#009E73` | 5,56 | 6,14 | 3,42 |
| bleu `#0072B2` (écarté) | 3,67 | 4,05 | 5,19 |

Deux conséquences : la palette `cvd` doit être **opaque** (sur fond blanc le jaune tombe à 1,3:1) ; les couleurs ANSI nommées ne sont **pas certifiables** (valeurs xterm : rouge `#800000` 1,92:1 et bleu `#000080` 1,31:1 sur noir) : la palette `default` n'emploie que des variantes vives et des attributs, et `high-contrast` est le remède documenté. Aucune simulation de daltonisme n'a été exécutée `[H]` ; Okabe-Ito est la palette publiée pour cet usage `[D]`.

### 5.2 Jauges et états

| ID | Exigence | Test | Spike |
|---|---|---|---|
| GAU-1 | Toute jauge porte `Nom valeur/max (mot de bande)` ; la barre n'est que décorative. Barre ASCII `[###---]` | S | ✔ (barre d'état, panneau, danger) |
| GAU-2 | Au moins une case pleine dès que valeur > 0 ; valeurs bornées dans [0, max] | U | ✔ |
| GAU-3 | Un passage de bande est annoncé par un événement `Alert` `Essential` (« Passage en vigilance ») | U | ✔ |

### 5.3 Mouvement et temps

| ID | Exigence | Test | Spike |
|---|---|---|---|
| MOT-1 | Pas de clignotement ni de scintillement (voir COL-4) ; aucun élément ne bascule sur minuterie | S | ✔ |
| MOT-2 | En veille, la TUI n'écrit aucun octet | P | ✔ |
| MOT-3 | Machine à écrire : `off`/`fast`/`normal`/`slow`, interruptible par toute touche, jamais bloquante pour la saisie, ne touche jamais à l'état, plafond 2 s par événement | U | ○ |
| MOT-4 | `--reduce-motion`, `--screen-reader` et le plain imposent `off` | U | ○ |
| MOT-5 | Aucune pause chronométrée : toute attente est un `Prompt::Continue` ou une saisie | L | ○ (`sleep`/`Instant` interdits dans le moteur) |
| MOT-6 | Aucun délai d'expiration sur une invite ; sauvegarde après chaque commande (déjà le cas en C) | U | ○ |

### 5.4 Mode lecteur d'écran (`--screen-reader`, `NEON_SCREEN_READER`, réglage)

| ID | Exigence | Test | Spike |
|---|---|---|---|
| LEC-1 | Implique le plain, `--no-color`, motion `off` ; jamais la TUI | P | ✔ (plain et sans couleur ; la motion n'est pas câblée) |
| LEC-2 | **Aucun octet ESC** (`0x1B`) dans toute la sortie, bannière et erreurs comprises | P | ✔ |
| LEC-3 | L'application n'émet ni `\r` isolé ni adressage du curseur (testé sur un tube) ; une sortie par commande, vidée immédiatement | P | ✔ |
| LEC-4 | Pas de décor : `Decor` omis (son `alt` seulement en `full`) | U | ✔ |
| LEC-5 | Variantes `clé@sr` sans symbole ambigu (`→ · × ¢ ✓ ○ █ ░ ─ │ ╭ • … »`) : « de 4 à 14 », « 80 crédits », « 40 sur 100, niveau vigilance » | U | ✔ |
| LEC-6 | Tableaux linéarisés avec le nom des colonnes : `1. Objet : Ghost Protocol ; Prix : 80 crédits ; État : disponible` | U | ✔ |
| LEC-7 | Les changements d'état sont annoncés en texte après chaque commande (Trace, bande, gain, nouveau courrier) | U | ✔ (Trace) |
| LEC-8 | Au premier lancement, une question en plain propose le mode d'affichage (plein écran / ligne par ligne / lecteur d'écran) et l'enregistre ; `options` la rouvre | P | ○ |
| LEC-9 | Protocole manuel avant chaque version : NVDA + Windows Terminal, Narrator, VoiceOver + Terminal.app, Orca + gnome-terminal, brltty + console Linux | M | ○ (non fait ici) |

### 5.5 Clavier

| ID | Exigence | Test | Spike |
|---|---|---|---|
| CLA-1 | Tout est faisable au clavier ; la souris n'ajoute rien | P | ✔ (périmètre du spike) |
| CLA-2 | `Échap` ferme toujours la modale, puis le tiroir, puis revient au bas du journal : aucun piège | P | ✔ |
| CLA-3 | TAB complète dans la saisie, ne change jamais de focus (il n'y a qu'un champ) | U | ✔ |
| CLA-4 | Chaque raccourci a une **commande** équivalente (`F2` ↔ `panel`) : certains terminaux ou claviers captent les touches de fonction | U | ○ |
| CLA-5 | AltGr (`CONTROL\|ALT`) insère le caractère ; Ctrl seul reste un raccourci | U | ✔ |
| CLA-6 | Ctrl+C quitte proprement, Ctrl+Z suspend et reprend (Unix) | P | ✔ |
| CLA-7 | `help` liste les raccourcis ; aucun chord de plus de deux touches | U | ○ |

Raccourcis : `Enter` valider · `Tab` compléter · `↑/↓` historique · `Ctrl+A/E/B/F/U/K/W` édition · `PgUp/PgDn` défiler · `F2` panneau · `Échap` fermer · `Ctrl+P` passer en plain · `Ctrl+Z` suspendre · `Ctrl+C` quitter.

### 5.6 Vocabulaire et cohérence

| ID | Exigence | Test | Spike |
|---|---|---|---|
| VOC-1 | Un **glossaire** (`data/glossary`) donne, par concept, le terme canonique FR/EN et les variantes interdites (Trace, Notoriété, crédits, cycle, deck, programme, nœud…) ; un test balaie le catalogue et échoue sur une variante interdite | L | ○ |
| VOC-2 | Les termes du glossaire sont injectés par `Arg::Term`, jamais écrits en dur dans une phrase | L | ○ (le mécanisme `Arg::Term` existe, le lint non) |
| VOC-3 | Un même fait a **un** gabarit : barre d'état, panneau **et** commande `status` partagent le même libellé de jauge | U | ✔ barre d'état + panneau (`gauge_label`) ; ○ `status` (le spike a un second gabarit `status.line`) |
| VOC-4 | Messages d'erreur : ce qui s'est passé, puis ce qu'on peut faire (« Commande inconnue : bogus. Tape help. ») ; tutoiement partout pour les contacts | M | ○ |
| VOC-5 | Les **noms de commandes restent en anglais** dans les deux langues (`scan`, `hack`, `shop`) et sont stables ; pas de capitales sur de longs textes (capitales seulement pour de courts libellés) | L | ○ |

### 5.7 Verbosité

| ID | Exigence | Test | Spike |
|---|---|---|---|
| VRB-1 | `brief` : `Essential` seul ; `normal` : + `Normal` ; `full` : tout, plus les alternatives de décor et les annonces de variation | U | ✔ |
| VRB-2 | `Error`, `Alert`, `Reward` et les invites sont toujours `Essential` (propriété du moteur, vérifiée sur une session) | U | ✔ |
| VRB-3 | Réglable par `--verbosity`, réglage enregistré et commande `verbosity` ; le lecteur d'écran n'est jamais en dessous de `normal` | U | ✔ (drapeau) |

### 5.8 ASCII

| ID | Exigence | Test | Spike |
|---|---|---|---|
| ASC-1 | `--ascii` = **7 bits stricts** : cadres `+-\|`, jauge `[###---]`, glyphes `[x]`/`[ ]`/`*`, `¢` → `cr`, `→` → `->`, accents translittérés. Tout octet de la sortie plain < 0x80 ; toute cellule de la TUI ASCII | U + S | ✔ |
| ASC-2 | Mise en page calculée sur le texte **final** (après translittération), via un point de passage unique | U | ✔ |
| ASC-3 | Activé automatiquement si la locale (`LC_ALL`/`LANG`) n'est pas UTF-8 (hors Windows) | P | ○ |
| ASC-4 | Aucun émoji dans le contenu (lint équivalent à `test_no_emoji_anywhere` du C) | L | ○ |
| ASC-5 | `--ascii` ≠ mode lecteur d'écran : le second garde les accents et retire le décor ; on peut combiner les deux | U | ✔ |

Largeurs mesurées avec `unicode-width 0.2.2` [E, `examples/widths.rs`] : « é » 1 ; `e`+U+0301 1 ; `¢` 1 ; `█ ─ ╭ → · … × • ○` 1 (mais **2** en `width_cjk`) ; `U+1F525` (flamme) 2 ; `U+2764` 1 ; `U+2764 U+FE0F` 2 ; drapeau (deux indicateurs régionaux) 2 ; famille (séquence ZWJ) 2 ; CJK 2 par caractère ; U+200B 0.

### 5.9 Export

| ID | Exigence | Test | Spike |
|---|---|---|---|
| EXP-1 | `--transcript FICHIER` : UTF-8, sans ANSI, ajouté et vidé à chaque commande (plain) | P | ✔ (contenu ; le vidage par commande est lu dans le code) |
| EXP-2 | Le transcript de la TUI est **identique** à celui du plain pour le même script | U | ✔ |
| EXP-3 | Commande `export [fichier]` en jeu ; transcript TUI écrit à la sortie et à l'export, pas à chaque touche | U | ○ |
| EXP-4 | Entrées enregistrables (`--record` JSONL) et rejouables (`--replay`) : même graine + mêmes entrées = mêmes octets | P | ○ |

### 5.10 Taille et localisation

| ID | Exigence | Test | Spike |
|---|---|---|---|
| TAI-1 | Plain : aucune taille minimale, aucune dépendance à la largeur du terminal (la sortie ne lit jamais la taille) | P | ✔ |
| TAI-2 | TUI : paliers 100×28 / 64×20 ; repli plain au démarrage avec message ; écran « trop petit » en cours de partie sans perte d'état | P + S | ✔ |
| TAI-3 | Aucun panic ni débordement à toute taille, de 0×0 à 300×100 | S | ✔ |
| LOC-1 | Parité FR/EN : mêmes clés, mêmes marqueurs, y compris les variantes `@sr` | U | ✔ |
| LOC-2 | Snapshots des paliers dans les **deux** langues (le français est ≈ 15-30 % plus long `[H]`) | S | ✔ (FR 100×30, EN 80×24, FR ASCII 64×20) |
| LOC-3 | Pluriel en catalogue, singulier pour 0 en français | U | ✔ |
| LOC-4 | Les confirmations acceptent les deux langues (`o`/`y`/`oui`/`yes`, `n`/`non`/`no`) quelle que soit la langue d'affichage | U | ○ |
| LOC-5 | Pas d'espace insécable fine dans les nombres (largeur et lecture variables) ; noms propres jamais traduits | L | ○ |
| LOC-6 | Changement de langue en partie : le journal est réécrit | S | ○ (par construction) |

---

## 6. Stratégie de test

### 6.1 Pyramide

| Niveau | Outil | Ce qu'il garantit | Spike [E] |
|---|---|---|---|
| Moteur | tests unitaires, `proptest`, `neon-sim` (autres tâches) | règles, invariants, complétude | 5 tests : mêmes événements pour une même graine, parité du catalogue, aucune clé manquante, rôles essentiels |
| Logique pure des frontends | unitaires | éditeur, wrap, palettes, précédence des options | 27 tests (`LineEditor` 8, `text` 4, `palette` 6, `plain` 8, `opts` 1) |
| Plain | transcripts `insta` + invariants | texte exact FR, texte exact EN/lecteur d'écran, 7 bits, zéro ESC, mêmes octets, `brief` | 2 snapshots + 6 invariants |
| TUI | `TestBackend` + `insta` | texte de l'écran à chaque palier, langue, ASCII, propriétés de style | 4 snapshots + 6 propriétés (aucune couleur en `mono`, aucun clignotement, fond opaque, jauge chiffrée, transcript, redimensionnement) |
| Bout en bout | `portable-pty` + `vt100` | binaire réel : écran alternatif, redimensionnement, panique, signaux, couleur, tubes, bascule | **14 tests**, ≈ 2 s |

Les snapshots texte ne voient pas les styles ; les propriétés de style se testent par balayage explicite du `Buffer` (aucune couleur en `mono`, aucun clignotement, fond opaque partout) plutôt que par un instantané illisible. Les matrices recommandées : 100×30, 80×24, 64×20, 50×15 × {fr, en} × {mono, ascii}.

**Politique `insta`** (mesurée [E]) : en local, un écart écrit `*.snap.new` et fait échouer le test ; `INSTA_UPDATE=always` accepte tout ; avec `CI=1` rien n'est écrit et le test échoue. `cargo insta review` (`cargo-insta 1.49.0`) n'est pas installé ici. Les snapshots sont versionnés ; un changement de `ratatui` ou de `unicode-width` les déplace, la version est figée par `Cargo.lock`, la mise à jour est une PR dédiée qui les valide.

### 6.2 Remplacer `pty_hud.py`

| `pty_hud.py` (Python) | Rust |
|---|---|
| `Screen` : mini-émulateur (curseur, région de défilement, effacements) | `vt100::Parser` : écran alternatif, collage, souris, attributs, redimensionnement |
| `pty.fork`, `select` | `portable-pty` (`openpty`, `spawn_command`, `resize`) |
| ~75 vérifications | `#[test]` dans `cargo test`, sans Python |
| HUD, menu, tutoriel, boutique, saisie TAB/historique | à porter *par comportement* (barre d'état, panneau, modale boutique, TAB, historique, redimensionnement) |

Squelette (extrait du spike) :

```rust
fn spawn(rows: u16, cols: u16, args: &[&str], env: &[(&str, &str)]) -> Session {
    let pair = native_pty_system().openpty(size(rows, cols))?;
    let mut cmd = CommandBuilder::new(BIN);          // env!("CARGO_BIN_EXE_neon-hack")
    cmd.env_clear(); cmd.env("TERM", "xterm-256color"); cmd.env("LANG", "C.UTF-8");
    let child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);                                // sinon le lecteur ne voit jamais la fin
    // un thread lit le maître et envoie des blocs : Linux renvoie EIO (pas 0) à la fermeture
}
fn wait_for(&mut self, what: &str, pred: impl Fn(&str) -> bool) -> String { /* boucle de 10 ms, délai 10 s, jamais de sleep fixe */ }
fn resize(&mut self, rows: u16, cols: u16) { self.master.resize(size(rows, cols))?; self.parser.screen_mut().set_size(rows, cols); }
```

Pièges [E] : lâcher l'esclave ; EIO en fin de flux ; attendre par **conditions** et non par `sleep` (10 exécutions consécutives de la suite de 14 tests, 0 échec ; avant cela 15 puis 12 exécutions de suites plus petites) ; synchroniser la taille de l'émulateur avec celle du pty ; un `Échap` seul n'est reconnu qu'après un court délai ; **`vt100` ne répond pas aux requêtes du terminal** (position du curseur), d'où l'interdiction de `Terminal::clear()` ; vérifier la restauration en lisant `alternate_screen()`, `hide_cursor()`, `bracketed_paste()`, `mouse_protocol_mode()`.

### 6.3 Déterminisme

- **Graine** : `Game::new(seed, …)` ; même graine, mêmes événements et mêmes octets [E] (unitaire et par tube : deux exécutions du binaire sont identiques et égales au fichier transcript [E]).
- **Aucune horloge** dans le moteur ; **aucun ordre de `HashMap`** observable (`BTreeMap`/`IndexMap`) `[H]`.
- **Environnement injecté** : `EnvSnapshot` lu une fois, passé aux fonctions pures ; `std::env::set_var` est `unsafe` en édition 2024 [E, `E0133`], donc les tests ne modifient jamais l'environnement du processus et les tests pty utilisent `env_clear()` puis posent `TERM`, `LANG`, `NO_COLOR` explicitement.
- **Présentation neutralisée en test** : machine à écrire `off`, taille fixée, dossier de données temporaire, `\n` seul.
- **Multi-OS** : les transcripts plain sont identiques partout (pas de largeur, pas de locale dans le texte) ; les snapshots `TestBackend` dépendent de la version de `unicode-width`, figée.

### 6.4 CI

| Plateforme | Exécuté |
|---|---|
| Linux | tout : unitaires, snapshots, pty, `clippy -D warnings`, `fmt --check`, lints d'accessibilité |
| macOS | idem `[H]` |
| Windows | build, unitaires, `TestBackend`, un test plain par tube ; **pas de pty** |
| Alerte précoce | `cargo check --target` Windows (msvc, gnu) et macOS depuis Linux : passe aujourd'hui [E] |

Lints d'accessibilité en CI : émojis, variantes de vocabulaire interdites, `Instant`/`SystemTime`/`sleep` dans `neon-engine`, `BLINK`, octets non ASCII en mode `--ascii`.

---

## 7. Spike : ce qui a été construit et mesuré

### 7.1 Versions exactes

Trouvées par `cargo info` le 2026-10-07 et résolues dans `Cargo.lock` ; compilées ensemble avec `rustc 1.97.0` / `cargo 1.97.0`, édition 2024.

| Crate | Version | Rôle | MSRV |
|---|---|---|---|
| `ratatui` | **0.30.2** | TUI (avec `ratatui-core 0.1.2`, `ratatui-widgets 0.3.2`, `ratatui-crossterm 0.1.2`) | 1.88 |
| `crossterm` | **0.29.0** | événements, terminal ; atteint par `ratatui::crossterm` | 1.63 |
| `unicode-width` | **0.2.2** | largeur d'affichage | 1.66 |
| `unicode-segmentation` | **1.13.3** | graphèmes (curseur, découpe) | 1.85 |
| `clap` | **4.6.7** | options (`derive`) | 1.85 |
| `anstream` | **1.0.0** | sortie couleur du plain (suppression, Windows) | 1.66 |
| `signal-hook` | **0.4.5** | Unix : restauration sur SIGTERM/HUP, suspension | 1.66 |
| `insta` | **1.49.0** | snapshots (dev) | 1.66 |
| `portable-pty` | **0.9.0** | pseudo-terminal (dev) | — |
| `vt100` | **0.16.2** | émulateur de terminal (dev) | 1.70 |
| `tui-input` | 0.15.5 | évaluée, non retenue (plan B) | — |
| `ratatui-textarea` | 0.9.3 | évaluée, rejetée | 1.86 |
| `tui-textarea` | 0.7.0 | rejetée : incompatible | 1.56 |
| `rustyline` / `reedline` | 18.0.1 / 0.52.1 | consultées via `cargo info`, non essayées | — |

MSRV du projet : **1.88**. Arbre d'exécution du spike : 82 crates distinctes dont 9 de macros procédurales ; binaire release 1,99 Mo (1,50 Mo après `strip`) ; `signal-hook` existe en deux copies (0.3.18 tirée par crossterm, 0.4.5 directe : négligeable). Reproduction :

```bash
cd spike-tui && cargo build --workspace && INSTA_UPDATE=no cargo test --workspace
cargo clippy --workspace --all-targets && cargo fmt --check
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc   # idem windows-gnu, aarch64-apple-darwin
cargo run --release -p spike-cli --example bench_draw
```

### 7.2 Résultats [E]

- **56 tests, 0 échec** : 27 unitaires du frontend, 14 pty, 10 de snapshots/propriétés de `Buffer`, 5 du moteur. `clippy --all-targets` sans avertissement, `fmt --check` propre.
- Compilation propre de tout le workspace : ≈ 10 s ; suite de tests complète à froid : ≈ 7 s (machine multicœur).
- `cargo check --workspace --all-targets` : `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `aarch64-apple-darwin` passent.
- Image de 100×30 : 110 µs avec 100 000 entrées au bas du journal ; 11 ms avec 20 000 lignes de remontée (re-wrap linéaire en la profondeur : à mettre en cache).

### 7.3 Ce que le spike contient

Un moteur factice (`spike-engine`, ≈ 980 lignes avec ses tests : PCG32, catalogue FR/EN avec pluriels et variantes `@sr`, 8 commandes, boutique, carte) et un `spike-cli` (plain, TUI, options, palettes, éditeur, garde du terminal, 3 fichiers de tests). Le moteur factice sert à prouver le **contrat**, pas à préfigurer le jeu.

### 7.4 Pièges rencontrés (écart avec ce qu'on attendrait)

| # | Attendu (versions antérieures) | Constaté | Preuve |
|---|---|---|---|
| 1 | `ratatui` est un seul crate qui dépend de `crossterm` | 0.30 est un **workspace** (`ratatui-core`, `ratatui-widgets`, backends séparés) ; `ratatui::crossterm` ré-exporte la bonne version : **pas de dépendance directe à `crossterm`** (sinon risque de deux versions incompatibles) | `cargo tree`, `lib.rs:481` [C+E] |
| 2 | `tui-textarea` convient | 0.7.0 exige `ratatui ^0.29` : deux ratatui dans l'arbre, `E0277` sur `Widget` et `From<Event>` ; successeur `ratatui-textarea` | [E] `conflict-check/` |
| 3 | `Frame::size()` | dépréciée (`#[deprecated]`) : `Frame::area()` ; `symbols::border::Set<'a>` a un paramètre de durée de vie ; `Layout::vertical([…]).areas(rect)` renvoie un tableau ; `Terminal::get_cursor()` aussi dépréciée au profit de `get_cursor_position()` | `[C]` + compilation [E] |
| 4 | `Paragraph::line_count` pour dimensionner | derrière `unstable-rendered-line-info` : wrap fait maison | [C] |
| 5 | `ratatui::init()` + `restore()` suffisent | `restore()` ne désactive ni souris, ni collage, ni curseur ; avec `panic = "abort"` seul le hook s'exécute | mutation [E] |
| 6 | `terminal.clear()` après une reprise | interroge le curseur (`ESC[6n`), échoue sans réponse | [E] |
| 7 | `ratatui::run(\|t\| …)` | existe depuis 0.30 (init + restore) mais ne rend pas souris/collage | [C] |
| 8 | insta : mises à jour « magiques » | `.snap.new` en local, rien d'écrit avec `CI=1`, test en échec dans les deux cas | [E] |
| 9 | `unicode-width` somme des caractères | 0.2.x gère les séquences : `U+2764 U+FE0F` = 2, drapeau = 2, séquence ZWJ = 2 ; `str::width` compte **1** pour les caractères de contrôle (ESC, tab) alors que `char::width` renvoie `None` : **assainir avant de mesurer** | `examples/widths.rs` [E] |
| 10 | clap `#[arg(env = "X")]` sur un booléen | `NEON_SCREEN_READER=1`, `=0`, `=yes` et `=` (vide) **font échouer le lancement** (« invalid value … [possible values: true, false] ») ; seul `true`/`false` passe | [E] → lire l'environnement soi-même, de façon indulgente (`1/true/yes/on`), jamais d'`env` clap sur un booléen |
| 11 | `anstream` 0.6 | 1.0.0 ; `AutoStream::new(w, ColorChoice::Never)` supprime les SGR | [E] |
| 12 | `std::env::set_var` | `unsafe` en édition 2024 | [E] |
| 13 | `self.f(self.rng.next())` | `E0499` : lier le tirage avant l'appel | [E] |
| 14 | `SIGTSTP` pour suspendre | ignoré dans un groupe de processus orphelin (harnais) : le processus ne s'arrête pas | mutation [E] |
| 15 | pty : lecture jusqu'à 0 | Linux renvoie `EIO` ; il faut lâcher l'esclave ; `vt100` ne répond à aucune requête | [E] |

### 7.5 Ce qui n'a PAS été vérifié

- Aucune exécution sous **Windows** ni **macOS** (compilation seule) : filtre `KeyEventKind::Release`, AltGr réel, ConPTY, Windows Terminal/conhost, `Ctrl+Break`.
- Aucun **lecteur d'écran réel** (le protocole LEC-9 reste à jouer) ; aucune simulation de daltonisme.
- `rustyline`/`reedline` et `cargo-insta` : versions lues, jamais exécutés.
- Non câblés dans le spike : `Prompt`/`Input`/`Step` et les menus, machine à écrire, `--reduce-motion`, `--wrap`, `--record`/`--replay`, première question de mode, commande `export`, autres modales que boutique et carte, bascule de langue en partie, cache des lignes wrappées.
- Aucune mesure de mémoire.

---

## 8. Décisions à valider (8)

| # | Question | Options | Recommandation |
|---|---|---|---|
| 1 | Où résoudre la langue ? | dans le moteur ; **au rendu** (clé + arguments typés) | **au rendu**, avec variantes `@sr`, parité FR/EN testée (§ 2.3) |
| 2 | Saisie : crate ou maison ? et le plain sur TTY ? | maison / `tui-input` / `ratatui-textarea` ; plain « cuit » / `rustyline` | **éditeur maison pur** pour la TUI (`tui-input` en plan B) ; plain en mode **cuit**, sans TAB (§ 3, 4.2) |
| 3 | Seuils de taille et repli | 80×24 unique (comme le C) ; **64×20 compact + 100×28 complet** | paliers 64×20 / 100×28 ; repli plain **au démarrage**, écran « trop petit » ensuite, `Ctrl+P` pour basculer (§ 4.1, 4.4) |
| 4 | Frontend par défaut et premier lancement | TUI si TTY ; plain par défaut ; **question au premier lancement** | TUI quand entrée et sortie sont des TTY assez grands, sinon plain ; **question de mode au premier lancement** (en plain), enregistrée (LEC-8) |
| 5 | Palettes livrées | une seule ; **`default` + `high-contrast` + `cvd` + `mono` automatique** | les quatre ; `high-contrast` et `cvd` **opaques** ; `mono` forcé par `NO_COLOR` ; Rgb seulement en truecolor (§ 5.1) |
| 6 | Souris | activée ; **désactivée par défaut** (`--mouse`) | désactivée : elle casse la sélection native ; molette seule, rien d'exclusif à la souris ; collage bracketé toujours actif (§ 4.7) |
| 7 | Jeu de caractères | accents conservés partout ; **`--ascii` = 7 bits stricts + auto hors UTF-8** | `--ascii` strict (décor **et** translittération), distinct du mode lecteur d'écran qui garde les accents et retire le décor (§ 5.8) |
| 8 | Socle technique | dépendance directe à `crossterm` ; **`ratatui::crossterm`** ; pty sur toutes les plateformes ; **pty Unix seulement** | MSRV **1.88** ; pas de `crossterm` direct ; tests pty `cfg(unix)`, Windows = `TestBackend` + tube ; `cargo check` Windows/macOS dès le premier lot (§ 4.6, 6.4) |

---

## 9. Checklist d'acceptation

À cocher avant de déclarer la TUI et l'accessibilité livrées ; chaque ligne renvoie aux exigences du § 5. ✔ = déjà automatisé dans le spike (à reporter dans le vrai code).

**Couleur et contraste**
- [ ] `NO_COLOR`, `--no-color`, `--color` dans la précédence voulue, plain et TUI (COL-1, COL-2) ✔
- [ ] Aucune couleur en `mono`, aucun clignotement, aucun fond non peint en palette opaque (COL-3, COL-4, COL-6) ✔
- [ ] Contraste ≥ 4,5:1 calculé pour chaque couleur de texte des palettes opaques (COL-7) ✔
- [ ] Rgb seulement en truecolor (COL-8)

**Information et jauges**
- [ ] Toute jauge a ses nombres et son mot, partout où elle apparaît (GAU-1) ✔
- [ ] Aucun état porté par la couleur seule (COL-5) ✔
- [ ] Changements de bande annoncés par un événement essentiel (GAU-3) ✔

**Mouvement et temps**
- [ ] Veille silencieuse (MOT-2) ✔
- [ ] Machine à écrire réglable, interruptible, plafonnée ; désactivée par `--reduce-motion` et en lecteur d'écran (MOT-3, MOT-4)
- [ ] Aucune pause chronométrée, aucune horloge dans le moteur, aucun délai sur les invites (MOT-5, MOT-6)

**Lecteur d'écran**
- [ ] Zéro octet ESC, aucune réécriture de ligne, sortie vidée par commande (LEC-2, LEC-3) ✔
- [ ] Variantes sans symboles ambigus, tableaux linéarisés, annonces d'état (LEC-5, LEC-6, LEC-7) ✔
- [ ] Question de mode au premier lancement (LEC-8)
- [ ] Protocole manuel joué sur NVDA, Narrator, VoiceOver, Orca, brltty (LEC-9)

**Clavier**
- [ ] Tout au clavier ; `Échap` sans piège ; chaque raccourci a une commande ; AltGr (CLA-1, CLA-2, CLA-4, CLA-5, CLA-7) ✔ (sauf CLA-4 et CLA-7)
- [ ] Ctrl+C, Ctrl+Z, panique, SIGTERM : terminal toujours rendu propre (CLA-6) ✔ ; le cas `panic = "abort"` n'a été vérifié qu'une fois, par expérience : à mettre en CI dès que le profil release le change

**Texte**
- [ ] Glossaire appliqué par lint ; un gabarit par fait ; commandes stables (VOC-1 à VOC-5)
- [ ] Verbosité : essentiels jamais masqués ; réglable (VRB-1 à VRB-3) ✔
- [ ] `--ascii` : 7 bits sur le plain et sur chaque cellule de la TUI ; auto hors UTF-8 ; aucun émoji (ASC-1 à ASC-5) ✔ (sauf ASC-3, ASC-4)

**Export et déterminisme**
- [ ] Transcript sans ANSI vidé à chaque commande ; identique entre TUI et plain (EXP-1, EXP-2) ✔
- [ ] `export`, `--record`/`--replay` (EXP-3, EXP-4)
- [ ] Même graine, mêmes octets, deux exécutions du binaire (§ 6.3) ✔

**Taille, langue, plateformes**
- [ ] Paliers, repli au démarrage, écran « trop petit », aucune panique de 0×0 à 300×100 (TAI-1 à TAI-3) ✔
- [ ] Parité FR/EN complète (marqueurs, `@sr`), snapshots dans les deux langues, pluriels (LOC-1 à LOC-3) ✔
- [ ] Confirmations bilingues, noms propres intacts, pas d'espace fine (LOC-4, LOC-5)
- [ ] CI : Linux complet ; macOS complet ; Windows build + `TestBackend` + tube ; `cargo check` des trois cibles (§ 6.4)

---

## Annexe : arborescence du spike

```
spike-tui/
  Cargo.toml                    workspace, resolver 3, édition 2024, rust-version 1.88, versions exactes
  conflict-check/               preuve que tui-textarea 0.7.0 ne compile pas avec ratatui 0.30.2
  crates/spike-engine/          moteur factice : catalogue FR/EN, Event/Prompt/Step, PCG32, 5 tests
  crates/spike-cli/
    src/{opts,text,plain}.rs    précédence d'options, wrap/ASCII/jauge, frontend plain
    src/tui/{mod,input,palette,glyphs,term}.rs
    tests/{tui_snapshots,pty_e2e}.rs
    examples/{widths,input_widgets,anstream_check,render_demo,bench_draw}.rs
```
