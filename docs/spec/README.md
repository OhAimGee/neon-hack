# Spécifications du jeu (phase P1)

Ces documents sont les **spécifications normatives** que la phase R2 implémente. Ils complètent [`docs/design/`](../design/README.md), sans le réécrire (règle de préséance R-0 de [`DECISIONS.md`](../design/DECISIONS.md)) : quand une spécification corrige un dossier, elle le dit en une ligne et c'est elle qui fait foi.

| Spécification | Contenu | Mise en œuvre |
|---|---|---|
| [`glossary.md`](glossary.md) | Le vocabulaire du jeu : un terme canonique par concept et par langue, les variantes interdites, comment on cite un terme | `data/glossary.toml`, `data/text/<langue>/terms.toml`, `neon_engine::text::glossary` : contrôlé par `cargo test` dès maintenant |
| [`commands.md`](commands.md) | La table unique des commandes du jeu complet (hub, intrusion, système), leurs arguments, leur ouverture, la correspondance avec les 29 commandes du C | R2 (hub), R4 (intrusion) |

Chaque spécification dit aussi ce qu'elle laisse ouvert et ce que le propriétaire doit valider ; la liste consolidée est dans [`DECISIONS.md`](../design/DECISIONS.md) (§ 5).
