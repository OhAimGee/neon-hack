# Protocole d'essai humain (R7)

Ce que les bots ne peuvent pas dire : combien de temps dure vraiment la campagne pour une personne,
si les intrusions se lisent, si l'on grille des intrusions, si l'on s'ennuie ou si l'on se bloque.
Les bots donnent un plancher et un plafond ([`neon-sim.md`](../spec/neon-sim.md) § 9) ; cet essai
mesure le reste, et recale les seuils des objectifs T5, T10 et T11 et la cadence de référence
(8 commandes par minute) avec une vraie partie.

- **Qui** : une personne qui joue la campagne, Normal, une fois, de bout en bout.
- **Combien de temps** : 4 à 5 h de jeu visées (T10), en cinq séances d'une heure environ.
- **Ce qu'on rapporte** : le relevé ([`RELEVE.md`](RELEVE.md)), les transcripts, et les chiffres
  de `scripts/playtest-summary.sh`.

## 1. Avant de commencer

1. Récupérer la dernière version : `git checkout main && git pull`, puis compiler une fois :
   `cargo build --release -p neon-cli`.
2. Un terminal d'au moins **100×28** (le panneau latéral est alors permanent), en UTF-8.
3. Copier [`RELEVE.md`](RELEVE.md) hors du dépôt (ou dans `docs/playtest/results/`), le remplir pendant la partie.
4. Avoir un chronomètre sous la main (celui du téléphone suffit). **On chronomètre le temps de jeu**,
   pas le temps passé à écrire le relevé : le mettre en pause pour cela.

## 2. Lancer la partie

Un dossier de données à part, pour ne pas toucher à vos propres sauvegardes, et une graine fixe
(la graine ne change que des détails ; elle rend deux essais comparables) :

```bash
# première séance : une nouvelle campagne, en Normal
cargo run --release -p neon-cli -- --lang fr --difficulty normal --seed 1 --new --data-dir ~/neon-essai

# séances suivantes : la partie est reprise toute seule (elle garde sa difficulté)
cargo run --release -p neon-cli -- --lang fr --data-dir ~/neon-essai
```

- Le jeu sauvegarde seul. **Ne pas utiliser `load`** pour effacer une erreur : si l'on recharge un
  point de contrôle après une intrusion grillée, le noter dans le relevé (c'est une information).
- `--lang en` si l'on veut tester les textes anglais ; une langue par essai.
- Terminal plus petit que 100×28 : le jeu reste jouable, mais le panneau devient un tiroir (`F2`).

## 3. Comment jouer

Jouer comme **une personne raisonnable qui découvre le jeu**, pas comme son concepteur :

- Ne pas utiliser les documents de conception ni le solveur (`neon-sim`), ne pas lire les fichiers de `data/`.
- Lire les prévisions et la trace avant de tenter un coup. Acheter ce qui semble utile.
- Garder la Notoriété basse (`laylow`) quand elle monte, comme on le ferait pour de vrai.
- Les intrusions se jouent **à la main**. La commande `skip` (qui prend tout le butin sans le
  jouer) est permise quand l'envie manque, mais **à noter** : chaque `skip` est un signe que
  l'intrusion n'amuse pas ou n'est pas comprise.
- `hint` est permis ; à compter. Une demande d'indice est un signe d'obscurité.
- Quand on ne sait plus quoi faire pendant **plus de 3 minutes**, le noter (heure, où, pourquoi)
  avant de continuer : c'est la mesure la plus utile de l'essai.
- Ne pas s'arrêter pour « réparer » le jeu : noter, continuer.

## 4. Les séances

Cinq séances, coupées à la fin d'un chapitre (le jeu a sauvegardé à chaque quête principale) :

| Séance | Chapitre | Quêtes principales |
|---|---|---|
| 1 | 1 et 2 | M01 à M04 |
| 2 | 3 | M05 à M07 |
| 3 | 4 | M08 et M09 |
| 4 | 5 | M10 à M12 |
| 5 | 6 et fin | M13, M14, l'épilogue |

Une séance peut être coupée en deux si elle dépasse 1 h 30 ; la reprendre telle quelle.
**À la fin de chaque séance, avant `quit`** : taper `export` (écrit `transcripts/transcript-<n>.txt`
dans `~/neon-essai`, sans jamais écraser un fichier), puis `status` et recopier la Notoriété, les
crédits et la réputation dans le relevé. Le transcript ne contient que la séance en cours : sans
`export` avant de quitter, elle est perdue pour les mesures.

## 5. Ce qu'on relève

Dans le relevé, pendant le jeu :

1. **Heure de jeu à chaque quête terminée** (chronomètre) ; l'heure de début et de fin de séance.
2. **Chaque intrusion** : le site, le résultat (réussie, grillée, `skip`), et si l'on a hésité.
3. **Chaque blocage** de plus de 3 minutes, **chaque `hint`**, **chaque `load`**.
4. **Après chaque chapitre** : tension (1 à 5), ennui (1 à 5), lisibilité (1 à 5), une phrase libre.
5. **Les erreurs de texte** ou d'interface (faute, texte brouillon `TODO`, affichage cassé), avec la ligne.

Après l'essai, `scripts/playtest-summary.sh ~/neon-essai/transcripts/*.txt` compte le nombre de
lignes tapées, de quêtes terminées, de `hint`, de `skip` et de `load`, et donne la Notoriété la plus
haute : les chiffres à comparer à ceux des bots sans les recopier à la main.

## 6. Ce qu'on compare

Les bots mesurés au moment de la rédaction (Normal, bot `prudent`, 8 plans représentatifs) :

| Mesure | Bot `prudent` | Seuil actuel | Ce que dit l'essai |
|---|---|---|---|
| Lignes tapées | 622 au médian | T10 : 500 à 1 000 | lignes ÷ minutes de jeu = **cadence réelle** (hypothèse : 8 par minute) |
| Durée de la campagne | ≈ 1 h 15 | 4 à 5 h visées (humain) | durée réelle chronométrée |
| Intrusions grillées | 0 ‰ | T5 : 0 à 25 % | part réelle des intrusions grillées |
| Notoriété maximale | 65 | T11 : médiane sous 70, jamais « Chassé » sauf 10 % | Notoriété maximale réelle |
| Crédits à la fin | ≈ 16 000 | T7 : étal à 124 % des revenus | crédits inutilisés, achats manqués |

Lire l'essai ainsi, et décider ensemble de ce qui change :

- **Durée** : sous 3 h ou au-dessus de 6 h → la cadence de référence (§ 6 de la spec) et T10 sont à
  recaler sur la cadence réelle ; trop long ou trop court est une question de contenu, pas de seuil.
- **Grillage** : sous 5 % → Normal est trop facile pour un humain (marge Normal de T4 à resserrer) ;
  au-dessus de 35 % → trop dur ; entre les deux, T5 reste.
- **Notoriété** : « Chassé » plus d'une fois → l'économie de Notoriété est trop sévère à Normal.
- **Blocages, `hint` et `skip`** : chaque blocage est une ligne de travail (texte, indice, règle) ;
  plus de 5 `skip` ou de `hint` par chapitre dit que les intrusions ou les quêtes sont obscures.
- **Argent** : une quête qui exige un objet qu'on n'a pas pu payer est un bug d'économie (T7).

## 7. Après l'essai

1. Remplir la dernière page du relevé (synthèse).
2. Donner : le relevé, les transcripts (`~/neon-essai/transcripts/`), la sortie de
   `scripts/playtest-summary.sh`. Les mettre dans le dépôt est possible mais pas obligatoire :
   `docs/playtest/results/AAAA-MM-JJ/` si on veut les garder.
3. La suite est un lot R7.x : recaler la cadence et les seuils (`data/sim/targets.toml`, spec),
   corriger les blocages, et rejouer `neon-sim` pour vérifier que les barrières P tiennent.

## 8. Limites

- Une seule personne, qui connaît le projet : l'essai mesure un joueur informé, pas un nouveau venu.
  Un deuxième essai avec quelqu'un qui n'a jamais vu le jeu vaut davantage que de refaire celui-ci.
- Une seule graine et une seule difficulté. Une séance facultative en Expert (chapitre 1 seulement,
  nouvelle campagne dans un autre `--data-dir`) dit si l'écart avec Normal se sent.
- Les accents, couleurs et la lisibilité dépendent du terminal : le noter dans le relevé.
