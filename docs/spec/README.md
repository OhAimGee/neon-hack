# Spécifications du jeu (phase P1)

Ces documents sont les **spécifications normatives** que la phase R2 implémente. Ils complètent [`docs/design/`](../design/README.md), sans le réécrire (règle de préséance R-0 de [`DECISIONS.md`](../design/DECISIONS.md)) : quand une spécification corrige un dossier, elle le dit en une ligne et c'est elle qui fait foi.

| Spécification | Contenu | Mise en œuvre |
|---|---|---|
| [`glossary.md`](glossary.md) | Le vocabulaire du jeu : un terme canonique par concept et par langue, les variantes interdites, comment on cite un terme | `data/glossary.toml`, `data/text/<langue>/terms.toml`, `neon_engine::text::glossary` : contrôlé par `cargo test` dès maintenant |
| [`commands.md`](commands.md) | La table unique des commandes du jeu complet (hub, intrusion, système), leurs arguments, leur ouverture, la correspondance avec les 29 commandes du C | R2 (hub), R4 (intrusion) |
| [`run-screen.md`](run-screen.md) | L'écran d'intrusion : contrat `RunView`, ordre des lignes, prévision, trois présentations d'un même tour (plain, français, lecteur d'écran + ASCII), panneau TUI à 30 colonnes, exigences RUN-1 à RUN-14 | R4 (contrat de `View` dès R2) |
| [`missions.md`](missions.md) | Le langage de missions (14 objectifs, 9 conditions, 9 effets), la règle « objectif déjà vrai à l'ouverture », les neuf contradictions de la bible révélées par l'exécution ; exécuté par [`neon_engine::content`](../../crates/neon-engine/src/content/mod.rs) (porté du spike en R2.1) ; la référence précise du langage est [`missions-language.md`](missions-language.md) | R2 (R2.1 fait) |
| [`neon-sim.md`](neon-sim.md) | Le banc d'équilibrage : bots, métriques, seuils chiffrés (barrières et objectifs), interface, intégration CI | R6 |
| [`solver.md`](solver.md) | Le solveur d'intrusion : verdict mesuré, rôle (test et CI contre exécution bornée), représentation, bornes, repli, constats de design, règles proposées ; s'appuie sur le spike [`spikes/solver`](../../spikes/solver/README.md) | R4 |

Chaque spécification dit aussi ce qu'elle laisse ouvert et ce que le propriétaire doit valider ; la liste consolidée est dans [`DECISIONS.md`](../design/DECISIONS.md) (§ 5).
