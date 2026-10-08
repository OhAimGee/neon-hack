# Spécification : le glossaire unique

> Phase P1, lot P1.1. Résout la contradiction 9 du [cross-check](../design/CROSS-CHECK.md) (vocabulaire divergent entre les dossiers) et rend contrôlable l'exigence VOC-1 du dossier TUI. **Cette spécification est déjà mise en œuvre** : le glossaire est dans `data/glossary.toml`, les termes dans `data/text/<langue>/terms.toml`, et `cargo test -p neon-engine` fait respecter toutes les règles ci-dessous.

## 1. Le principe

Un **concept** a un seul mot par langue. Le mot est écrit une fois, dans les catalogues de textes (`term.<id>`), et le code le cite par `Arg::Term("<id>")` : le français et l'anglais ne peuvent donc pas diverger, et renommer un concept est un changement d'une ligne.

Le glossaire ajoute ce qu'un catalogue ne dit pas : le **sens** du concept (une note pour les rédacteurs) et les **variantes interdites** (les mots que les dossiers de conception employaient et qui prêtent à confusion).

| Fichier | Rôle |
|---|---|
| `data/glossary.toml` | un `[[concept]]` par entrée : `id`, `note`, `strict` (facultatif), `forbid` par langue (facultatif) ; réglages des préfixes de clé |
| `data/text/en/terms.toml`, `data/text/fr/terms.toml` | le mot canonique de chaque concept, clé `term.<id>` |
| `crates/neon-engine/src/text/glossary.rs` | lecture du glossaire et contrôle des catalogues (`Glossary::check`) |

## 2. Ce que le test contrôle

`cargo test -p neon-engine` échoue, avec la langue et la clé, si :

1. un concept n'a pas son terme dans une langue, ou un terme n'a pas de concept ;
2. deux concepts portent le même mot dans une langue ;
3. une variante interdite est elle-même le mot d'un concept ;
4. un texte de l'interface emploie une variante interdite (mot entier, casse ignorée, expression de plusieurs mots cherchée telle quelle) ;
5. le fichier du glossaire est mal formé (id en majuscules ou répété, langue inconnue, variante vide, champ inconnu).

**Où les variantes sont cherchées.** Dans les textes de l'interface (aide, jauges, boutique, erreurs…). Les textes de **narration** (clés commençant par `quest.`, `dialogue.`, `document.`, `mail.`, `scene.`, `ending.`) échappent aux variantes interdites, sauf pour les concepts marqués `strict = true` : un auteur doit pouvoir écrire « la chaleur monte » dans un dialogue sans que le test proteste, mais « yen » ne doit jamais y apparaître. Les étiquettes de rôle `ui.tag.*` (`[ALERT]`…) sont exemptées : ce sont des rôles d'événement, pas des mots du jeu.

## 3. Comment on utilise un terme

- **Dans une phrase** : on ne tape jamais le mot, on cite le terme (VOC-2).

  ```toml
  [demo.gauge]
  line = "{gauge} : {value}/100."          # gauge = Arg::Term("trace")
  ```

  ```rust
  Text::new("demo.gauge.line").with_term("gauge", "trace").with_int("value", 40)
  ```

- **Casse** : les termes sont écrits comme au milieu d'une phrase (« nœud », « intrusion »), sauf les noms propres et sigles (ICE, Intel, Trace, Notoriété et ses bandes, difficultés). Une phrase qui commence par un terme le met en majuscule **avec le marqueur `{nom^}`** : le suffixe `^` met en majuscule la première lettre de la valeur rendue (Unicode, indépendant de la langue), sans second texte à tenir à jour (`{gauge^} : {value}/100.`). La syntaxe `{nom^}` est à ajouter à `text::template` en R2 (le marqueur est déjà réservé : le suffixe est une erreur de gabarit aujourd'hui) ; d'ici là, le jeu jouet recopie « Handle » à la main, ce qui est la seule entorse à VOC-2 et disparaît avec lui.
- **Pluriel** : un terme est au singulier. Un pluriel régulier s'écrit dans le gabarit, avec le terme en marqueur imbriqué : `{n|{node}|{node}s}` (anglais et français : `node`/`nodes`, `nœud`/`nœuds`). Un pluriel irrégulier (« journal » → « journaux », « niveau » → « niveaux ») a son propre texte ; si le besoin se répète, R2 ajoutera la variante de clé `term.<id>@plural` à `text::check` plutôt que de multiplier les concepts.
- **Noms propres du monde** (contacts, lieux, entreprises, objets de boutique) : ils ne sont pas dans le glossaire. Ils vivent dans le contenu (`contact.<id>.name`, `site.<id>.name`…), ne se traduisent pas, et les règles de casse de la bible (§ 2.6) s'appliquent : IA en capitales, leet pour R4Z0R seul.
- **Noms de commandes** : jamais traduits, jamais dans le glossaire (voir [`commands.md`](commands.md)).

## 4. Ajouter ou renommer un concept

1. Ajouter le `[[concept]]` dans `data/glossary.toml` (id, note, variantes interdites s'il en existe de connues).
2. Ajouter `<id> = "…"` dans `data/text/en/terms.toml` **et** `data/text/fr/terms.toml`.
3. `cargo test -p neon-engine` : le test dit ce qui manque.

Renommer un **mot** (changer « Trace » en autre chose) ne touche que `terms.toml` et, si l'ancien mot doit disparaître, le met en `forbid`. Renommer un **id** après publication demande une migration de sauvegarde seulement si l'id est écrit dans une sauvegarde : les ids du glossaire ne le sont pas.

## 5. Arbitrages du cross-check 9

Le cross-check demande de trancher cinq divergences ; voici ce qui est écrit dans le glossaire, validé par le propriétaire le 8 octobre 2026 (S-1 de [`DECISIONS.md`](../design/DECISIONS.md) § 5).

| Divergence | Décision | Pourquoi |
|---|---|---|
| « alerte » / « chaleur » / Trace / Notoriété | **Trace** (jauge d'une intrusion, 5 bandes : calme, vigilance, alerte, danger, grillé) et **Notoriété** (jauge de campagne, 4 bandes : Discret, Surveillé, Traqué, Chassé). « Chaleur » est un terme de conception qui ne s'affiche jamais ; « niveau d'alerte » et « jauge d'alerte » sont interdits | une jauge a un nom, une durée de vie et un effet ; le mot « alerte » ne reste que comme nom d'une bande de Trace |
| « site » / « système » / « nœud » | **site** = lieu de la carte du monde (`net`) ; **nœud** = sommet du graphe d'une intrusion (`map`). « Système » et « serveur » ne nomment plus un lieu de jeu | c'est la règle du cross-check, reprise telle quelle |
| « Sentinelle » (ICE) / SENTINEL (division de Nexus) | l'ICE mobile s'appelle **patrouille** (*patrol*) ; **SENTINEL**, en capitales, reste la division de Nexus. « Sentinelle » est interdit en français | une seule des deux choses garde le mot ; l'ICE est la plus fréquente à l'écran, elle a donc le mot le plus court à lire |
| « niveau » / « palier » | **niveau** à l'écran (Novice → Légende) ; « palier » reste un terme de conception, jamais affiché et interdit dans l'interface | la bible fixe « niveau » ; le niveau n'est plus lié à l'XP mais aux quêtes principales, ce qui ne change pas le mot |
| quête / mission / contrat / journal | **quête** = entrée du journal de quêtes ; **contrat** = quête annexe qui porte un avantage (« quête annexe » interdit) ; **mission** = le mot des personnages, jamais une étiquette d'interface ; **journal** = le journal de quêtes (le panneau d'événements de la TUI s'appelle **terminal**) | le dossier TUI emploie « journal » pour le panneau d'événements et la bible pour le journal de quêtes : un seul des deux garde le mot |

Autres choix du glossaire : *run* en anglais, **intrusion** en français (« run » interdit en français) ; **handle** dans les deux langues (« pseudo » interdit : le jeu jouet disait « pseudo », il a été corrigé avec ce lot) ; **message** et **boîte de réception**, jamais « mail », « courriel » ni « courrier » dans l'interface ; **le réseau** / *the net*, jamais « la toile » ni « Internet » ; **crédit** (¢), jamais « yen » ; **prévision** / *forecast* pour la ligne qui annonce la fin du tour.

## 6. Le tutoiement et le vouvoiement (D-09), tel que ce lot le laisse

Les textes de l'interface déjà écrits (R1) s'adressent au joueur au **vous** (« Reprise de votre partie sauvegardée »), alors que le dossier TUI (VOC-4) prévoit le **tu** pour les messages d'erreur et que la bible fixe le tutoiement dans l'underground et le vouvoiement pour AURA, le Courtier et Voss. Proposition (S-2 de `DECISIONS.md` § 5, liée à D-09) : **l'interface est neutre** (formes impersonnelles ou infinitif : « Partie sauvegardée », « Commande inconnue : bogus. Voir `help`. ») ; **chaque personnage garde sa voix**. Aucun texte n'est réécrit tant que D-09 n'est pas tranchée ; le jeu jouet de R1 sera remplacé de toute façon en R2.

## 7. Limites

- Le contrôle cherche des **mots**, pas des sens : il ne détecte pas « niveau de danger » employé pour la Trace, et une variante interdite très courte produirait des faux positifs (c'est pourquoi les variantes sont des mots ou des expressions précises).
- Le glossaire ne couvre pas encore les noms de programmes, d'objets et de sites : ils arrivent avec le contenu (R2) sous forme de clés `program.<id>.name`, etc., avec leurs propres règles de casse.
- Les budgets de largeur d'affichage des termes (panneau de 30 colonnes utiles) sont contrôlés par le schéma de contenu de R2, pas ici.
