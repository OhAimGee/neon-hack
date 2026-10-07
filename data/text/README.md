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

## Vérifier

```bash
cargo test -p neon-engine        # charge les catalogues et applique toutes les règles
```

Le test `the_embedded_content_follows_every_rule` liste chaque infraction avec la langue et la clé. Les budgets de largeur d'affichage par famille de texte arriveront avec le schéma de contenu (phase R2).
