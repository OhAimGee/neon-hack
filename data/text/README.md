# Textes du jeu

Un dossier par langue (`en/`, `fr/`), les mêmes fichiers dans chacun. Le moteur les embarque à la compilation (`crates/neon-engine/build.rs`) : ajouter un fichier `.toml` dans un dossier de langue suffit, sans toucher au Rust.

## Écrire un texte

Les tables TOML s'aplatissent en clés pointées : `[ui.tag]` et `alert = "[ALERT]"` donnent la clé `ui.tag.alert`. Le code n'écrit jamais de phrase : il cite une clé et ses arguments, et la langue est choisie à l'affichage.

```toml
[demo.scan]
found = "The scan finds {found} open {found|port|ports}."
reward = "+{credits} {credits|credit|credits}."
```

- **Marqueur** : `{nom}` (lettres minuscules, chiffres, `_`) est remplacé par l'argument du même nom. Un nombre s'écrit en décimal, un texte tel quel.
- **Pluriel** : `{n|singulier|pluriel}` choisit une forme d'après le nombre `n`. Une forme peut contenir des marqueurs, `n` compris : `{n|un nœud|{n} nœuds}`, ou `{n||s}` pour ne rien écrire au singulier. Règle par langue : en français **0 et 1** sont singuliers (« 0 crédit »), en anglais seul **1** l'est.
- **Majuscule** : `{nom^}` est `{nom}` avec sa première lettre en majuscule (Unicode, quelle que soit la langue) : une phrase qui s'ouvre sur un terme du glossaire, que les catalogues tiennent en minuscules (`{quest^} terminée : {title}`). Le marqueur ne s'applique pas à un pluriel.
- **Accolades** : réservées aux marqueurs, sans échappement. Ce que le joueur tape (son pseudo) n'est jamais dans un texte : il arrive en argument.
- **Une clé ne peut pas être à la fois un texte et une table** : `demo.item.proxy.label` et `demo.item.proxy.name`, pas `demo.item.proxy` avec `demo.item.proxy.name`.

## Variantes

Une clé peut avoir des variantes, que le mode d'affichage préfère au texte de base :

- `"clé@sr"` : formulation pour lecteur d'écran, sans symbole à épeler (`→` devient « de 4 à 14 ») ;
- `"clé@ascii"` : formulation pour `--ascii` quand la simple translittération ne suffit pas (`»` devient `:`).

Les deux modes se combinent (`--screen-reader --ascii`) : la variante `@sr` passe d'abord, puis `@ascii`, puis le texte de base, et le résultat est translittéré. Une variante porte **les mêmes marqueurs** que sa clé de base. Un texte qui utilise un symbole mal lu par les lecteurs d'écran (`→ ← ↔ ¢ × · • ≤ ≥`) doit avoir sa variante `@sr`. Les noms de variante s'écrivent entre guillemets en TOML (`"say@sr" = …`).

## Règles de contenu

- Mêmes clés et mêmes marqueurs dans toutes les langues : l'anglais, première langue de `Lang::ALL`, sert de référence, et chaque autre langue est comparée à lui.
- Aucun texte vide ; une seule ligne (pas de retour à la ligne) ; pas d'espace insécable, pas de `…` (écrire `...`), pas d'émoji.
- `--ascii` translittère tout le texte affiché (`é` devient `e`, `«` devient `"`, `→` devient `->`, `¢` devient `cr`, ce qui n'a pas d'équivalent devient `?`) : il n'y a donc pas besoin d'une variante `@ascii` pour les accents.

## Vocabulaire

Les mots du jeu (Trace, Notoriété, nœud, quête…) ne s'écrivent pas à la main dans une phrase : `terms.toml` en donne le mot canonique (`term.<id>`), le code le cite par `Arg::Term`, et `data/glossary.toml` liste les variantes interdites. Règles et arbitrages : [`docs/spec/glossary.md`](../../docs/spec/glossary.md).

## Les fichiers du jeu de campagne

- `campaign.toml` : les textes d'**interface** du hub (aide, fiches, listes, invites, résultats, raisons d'ouverture `unlock.*`, services `service.*`). Neutres en français (ni tu ni vous), sans terme du glossaire tapé : le jeu donne au gabarit les termes qu'il cite (`{quest}`, `{net^}`).
- `world_names.toml` : les noms propres des contacts et des objets (clés `contact.<id>.name`, `item.<id>.name`).
- `l1_*.toml` : les textes du lot L1 (chapitres 1 et 2), par famille : `l1_quests` (M01-M04), `l1_contacts` (accroches et sujets), `l1_mails`, `l1_documents` (fragments F01-F06 et titres des documents chiffrés), `l1_cutscenes` (CS02), `l1_world` (sites, fichiers, objets). Les lots suivants suivent le même patron (`l2_*.toml`...). Un texte d'histoire ne reçoit que les termes du glossaire que le jeu ajoute à tout texte (`{net}`, `{quest}`, `{journal}`, `{notoriety}`, `{credit}`...) : ni `{handle}` ni autre marqueur, un test le contrôle ; les commandes s'écrivent entre apostrophes inversées. Les tables numérotées `[quest.m01.obj]` et `[quest.m01.hint.1]` donnent `quest.m01.obj.1` et `quest.m01.hint.1.1`.
- `world_draft.toml` : un brouillon `TODO <clé>` pour chaque clé de texte **dérivée des données** qu'aucun autre fichier ne définit (titres, objectifs, dialogues, messages, fragments...). **Généré, ne pas éditer** : pour écrire un texte, définir sa clé dans un vrai fichier du dossier, puis régénérer (`NEON_REGEN_WORLD_DRAFTS=1 cargo test -p neon-engine --test content regenerate_the_draft_texts -- --ignored`) ; le brouillon disparaît. Les brouillons sont exemptés des budgets de largeur ; `the_number_of_draft_texts_is_reported` (`--nocapture`) donne le compte par langue, et `no_draft_text_remains` (ignoré) échouera tant qu'il en reste : à dégriser à la fin de R5.

## Vérifier

```bash
cargo test -p neon-engine        # charge les catalogues et applique toutes les règles
```

Le test `the_embedded_content_follows_every_rule` liste chaque infraction avec la langue et la clé. Les budgets de largeur d'affichage par famille de texte (`content::texts::budget`) sont contrôlés sur les textes écrits.
