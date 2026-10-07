# Audit des systèmes de jeu et cahier de refonte

> Livrable de la tâche A (audit des mécaniques + proposition de redesign pour la réécriture en Rust).
> Dépôt audité : branche `claude/relaxed-lamport-87hbbr`, HEAD `653bc46`, arbre propre. Rien n'a été modifié dans le dépôt en dehors de ce fichier.
> Les termes de jeu (commandes, objets, noms de systèmes) restent en anglais comme dans le code ; le reste est en français.

**Légende des preuves** — `[E]` mesuré en exécutant le code (jeu C compilé dans une copie, voir l'annexe A) ; `[C]` lu dans le code (`fichier:ligne`) ; `[H]` hypothèse ou estimation, à confirmer.

## 0. Résumé

1. **Tout le contenu actuel s'épuise en une centaine de commandes** `[E]`. Un bot « complétiste » (tutoriel, 4 quêtes, 7 systèmes, tous les jalons) joue 98 commandes en moyenne (500 graines) et atteint le niveau 6 à la 33e. Un joueur qui ne tape que `scan` et `bruteforce` atteint le niveau 6 et compromet les 7 systèmes en ≈ 29 commandes, tutoriel de 10 commandes compris.
2. **« Taper bruteforce jusqu'à ce que les dés réussissent » : en pratique, une seule frappe suffit.** `bruteforce` fait 5 essais par commande : 99,5 / 96,9 / 88,4 / 67,2 % de réussite par commande selon la sécurité 1 à 4, quel que soit le niveau `[E]`. Les autres méthodes sont dominées, ou ne servent qu'à toucher leur prime de crédits.
3. **L'alerte est neutralisée** par `laylow` → 1 « Attendre » : gratuit, illimité, −8. Sans lui, 75 % des parties finissent en game over ; avec lui, 0 %, pour ≈ 8 commandes d'attente par partie (28 % des commandes) `[E]`. C'est une taxe en frappes, pas un risque.
4. **L'économie est cassée** : ≈ 30 700 ¢ de revenus uniques contre 2 175 ¢ de boutique. Une seule commande (`quantumdecrypt CLASSIFIED`, mot jamais affiché au joueur) verse 12 000 ¢ `[C]`.
5. **Beaucoup de mécaniques sont mortes ou trompeuses** : `stealthmode` ne bascule rien, `aiassist` n'applique aucun bonus, `firewall_strength` n'entre dans aucune formule, les batteries d'outils ne se rechargent jamais, le contenu des fichiers n'est jamais affiché, 5 contacts sur 9 et 6 quêtes sur 10 n'existent pas (section 2.2).
6. **Un monde de 7 systèmes ne peut pas porter la campagne** : le niveau 6 arrive au tiers du contenu, au moins 3 systèmes sur 7 sont déjà compromis quand s'ouvre la 4e quête (la 3e en exige 3), et il n'existe ni documents, ni drapeaux d'histoire, ni choix, ni boss (section 2.7-2.8).
7. **Recommandation** : la **direction A, « Infiltration à tours »** (section 3) : des intrusions tour par tour sur de petits graphes rédigés à la main, à résolution déterministe et menaces annoncées à l'avance, une jauge de Trace par intrusion et une Notoriété persistante, des missions écrites comme des données. Choisie pour la TUI (carte, jauge, deck lisibles), l'accessibilité (tour par tour, tout est texte) et la **complétude démontrable** : un solveur exact vérifie qu'une mission se termine (spike Rust : 8 393 états, 3 à 5 ms pour une mission jouet de 6 nœuds, même optimum qu'une recherche exhaustive de 3 millions d'états `[E]`). Les runs sont derrière une interface (résolution tactique ou automatique) : on livre d'abord la campagne complète avec résolution automatique, puis on y met la profondeur tactique.
8. **Ordre de grandeur** `[H]` : moteur Rust 9,5 à 15 k lignes (+ tests du même ordre), frontend plain 0,7 à 1 k, TUI 2 à 3 k, harnais d'équilibrage 0,8 à 1,3 k ; ≈ 29 000 mots de texte français à écrire (+ l'anglais en parité) contre ≈ 2 200 mots narratifs aujourd'hui.
9. **12 décisions à valider** (section 6).

---

## 1. Catalogue des mécaniques actuelles

Source : `src/game/*.c`, `src/i18n/strings.def`, tests `tests/unit/*`. Tous les chiffres ci-dessous sont ceux du code, recoupés par exécution quand c'est indiqué `[E]`.

### 1.1 Commandes

29 entrées dans `k_commands` (`commands.c:149-183`) : 10 de hacking, 7 « monde », 7 de hacking avancé, 5 système. Déblocage = niveau minimum **et** drapeau `CMD_*` (`commands.c:210-217`). Le temps passe (refroidissement de l'alerte −1, −2 avec VPN) avant **toute** commande des catégories HACK et ADVANCED, y compris les commandes sans effet (`commands.c:250-256`).

| Commande | Déblocage | Effet exact | Alerte (succès / échec) | Récompense |
|---|---|---|---|---|
| `scan` | départ | révèle les systèmes dont le niveau minimum ≤ niveau du joueur ; animation 1,5 s | +1 | XP 5, 4, 3, 2, 1 pour les 5 premiers scans, puis 0 |
| `bruteforce <sys>` | niveau 2 | cible découverte + relais compromis ; **5 essais indépendants** dans une commande (`cmd_hacking.c:103-128`) ; chance 80 − 15×sécurité | +5×sécurité / +8×sécurité (une fois, si les 5 échouent) | compromission (1.7) |
| `decrypt <texte>` | niveau 3 | César −3, `#` = espace ; « THIS IS A TEST » donne le jalon | aucune | +20 XP, une fois |
| `backdoor <sys>` | niveau 3 | relais requis ; chance 70 + 10×niveau − 15×sécurité (+20 si mode furtif caché) ; animation **38 s** `[E]` | 10 (5 si furtif) / 20 | **+500 ¢** et +15 XP, une fois par système (`cmd_hacking.c:221`) |
| `traceroute <sys>` | niveau 4 | passif (aucun relais requis) ; affiche corporation, firewall, nb de fichiers, relais ; marque « tracé » ; 7,5 s | +3 | +8 XP, une fois par système |
| `exploit <sys>` | niveau 4 | exige `traceroute` d'abord, ignore les relais ; chance 60 + 5×niveau − 12×sécurité ; 24 s | 12 / 25 | compromission |
| `uploadvirus <sys>` | niveau 4 | relais requis ; **virus tiré au hasard** parmi la bibliothèque (1 / 2 / 3 virus aux niveaux 3 / 4 / 5) ; chance 60 − 10×sécurité + furtivité du virus (8 / 5 / 3) + 30 si backdoor ; 33 s | 15 − furtivité / 25 | +20 XP, +10×dégâts ¢ (250 / 400 / 700), une fois par système (`cmd_hacking.c:386`) ; baisse un « firewall » sans effet de jeu |
| `stealth` (caché) | furtivité ≥ 5 | bascule `stealth_mode` : +20 à `backdoor`, alerte de `backdoor` 10 → 5, active l'outil « Cape » ; annonce un coût « −1 furtivité par minute » qui n'existe pas | — | — |
| `aihack <sys>` | niveau 5 **et** IA achetée | relais requis ; chance 85 + 5×niveau (−20 si sécurité critique) ; succès = compromission **profonde** (tous les fichiers) ; sur un système déjà compromis, extrait les fichiers restants ; 34 s | 5 / 30 (5 pour l'extraction) | compromission + fichiers |
| `quantumdecrypt <données>` | niveau 5 **et** puce achetée | ne peut pas échouer ; si le texte contient `CLASSIFIED` : jalon ; 48 s | 2 | 1re fois +50 XP et +2 000 ¢ ; 1re fois avec `CLASSIFIED` **+10 000 ¢** (`cmd_hacking.c:529,564`) |
| `shop` | départ (aucun contact requis) | catalogue de R4Z0R (1.6) ; fermé si alerte ≥ 80 | — | — |
| `laylow` | départ | menu de réduction d'alerte (1.5) | — | — |
| `quests`, `contacts`, `contact <n° ou nom>`, `messages`, `read <n>` | départ | journal, liste, conversation à menu, boîte de réception | — | — |
| `advhack <sys>` | niveau 3 | menu de 8 méthodes (1.10), relais requis, système avec profil avancé (5 sur 7) | 0 / voir 1.4 | compromission profonde |
| `aiassist <sys>` | niveau 3 | n'exige même pas l'IA ; affiche un bonus et « configuré pour la prochaine tentative » mais **n'applique rien** | — | — |
| `socialeng <sys>` | niveau 2 | système avec profil avancé + relais ; chance 50 + 5×niveau + **2×réputation** − sécurité/2 (5 à 95) | — / +10 | +20 XP une fois par système ; +15 de chance sur ce système (`has_intel`) |
| `stealthmode` | départ | **ne bascule rien** : `StealthSystem.is_active` n'est jamais mis à vrai `[C]` `cmd_advanced.c:113-131`, `[E]` | — | refroidit l'alerte (catégorie ADVANCED) |
| `analyzedefenses [sys]` | départ | affiche les défenses ; sans argument, liste les cibles avancées ; ne demande pas de relais | — | refroidit l'alerte |
| `neuralsync` | niveau 5 | +10 de synchronisation (max 100), gratuit, sans limite ; ≥ 50 active l'outil « interface neurale » ; l'« entraînement de l'IA » ne change rien (division entière, `advanced_hacking.c:802-819`) `[E]` | — | — |
| `temporalhack <sys>` | niveau 6 **et** synchronisation ≥ 90 | sans relais ; chance 30 + 5×niveau (60 % au niveau 6) | — / +20 | compromission profonde |
| `status`, `help`, `save`, `quit` (`exit`), `clear` | départ | système | — | — |

`analyzedefenses` et `stealthmode` figurent dans `help` dès le niveau 1 `[E]` : ce sont des commandes de refroidissement gratuit (1.5).

### 1.2 Niveaux et expérience

Courbe cumulée `{0, 0, 15, 60, 140, 260, 420}` (`progression.c:12`), 6 niveaux. Un seul point d'entrée, `nh_grant_xp`.

| Niv. | Nom | XP cumulée | Commandes ouvertes (hors drapeau) | Virus | Furtivité | Systèmes révélés par `scan` | Objets ouverts (prix) | Quête ouverte |
|---|---|---|---|---|---|---|---|---|
| 1 | Novice | 0 | `scan`, boutique, `laylow`, contacts…, `stealthmode`, `analyzedefenses` | 0 | 3 | localhost | Ghost Protocol (80), Street Cred (75), VPN (60) | tutoriel |
| 2 | Apprenti | 15 | `bruteforce`, `socialeng` | 0 | 3 | corp-server-01 | Stealth Module (150), Proxy Chain (100), Accelerator (90) | Baptême du Feu |
| 3 | Hacker | 60 | `decrypt`, `backdoor`, `advhack`, `aiassist` | 1 | 3 | nexus-mainframe, underground-market | Encryption Key (200) | Réseaux d'Information |
| 4 | Expert | 140 | `exploit`, `traceroute`, `uploadvirus` | 2 | 5 | research-lab, banking-network | Malware Arsenal (120), Neural Assistant (500) | L'œil du Cyclone |
| 5 | Maître | 260 | `aihack`, `quantumdecrypt`, `neuralsync` | 3 | 8 | gov-database | Quantum Chip (800) | aucune |
| 6 | Légende | 420 | `temporalhack` | 3 | 8 | — | — | aucune |

`k_rewards` (`progression.c:100-108`) : déblocages, bibliothèque de virus, bonus de furtivité (+2 au niveau 4, +3 au niveau 5). Départ : 100 ¢, réputation 0, furtivité 3 (`game.c:54-97`).

### 1.3 Le réseau (7 systèmes, `world.c:44-69`)

```
localhost ─┬─ corp-server-01 ─┬─ nexus-mainframe ─ gov-database
           │                  └─ research-lab
           └─ underground-market ─ banking-network
```

| Système | Corporation | Sécu. | ¢ à la 1re compromission | XP | Niv. scan | Relais | Fichiers (chiffrement : ¢) | Profil avancé (sécurité /100 ; défenses) |
|---|---|---|---|---|---|---|---|---|
| localhost | Independent | 1 | 10 | 5 | 1 | — | user_data.txt (1 : 50) | — |
| corp-server-01 | MegaCorp Industries | 2 | 50 | 25 | 2 | localhost | employee_records.db (2 : 200), financial_data.xlsx (3 : 500) | — |
| nexus-mainframe | Nexus Corp | 3 | 200 | 100 | 3 | corp-server-01 | project_ghost.dat (4 : 1 000), neural_maps.bin (5 : 1 500), quantum_keys.qkey (6 : 2 000) | 95 ; AURA Defense Grid 90, Quantum Firewall 95, BehaviorScan Pro 80 |
| underground-market | Underground | 2 | 150 | 30 | 3 | localhost | black_ledger.dat (2 : 300) | 55 ; BasicWall 55 |
| research-lab | TechDyne Research | 3 | 350 | 60 | 4 | corp-server-01 | prototype_specs.cad (4 : 600) | 70 ; LabWatch AI 70, DecoyNet 60 |
| banking-network | MegaCorp Financial | 3 | 600 | 80 | 4 | underground-market | vault_keys.enc (5 : 900) | 85 ; BankGuard Firewall 85, SIEM Advanced 80 |
| gov-database | Gouvernement de Neo-Tokyo | 4 | 800 | 110 | 5 | nexus-mainframe | citizen_registry.db (6 : 1 200) | 90 ; SentinelAI MK-VII 88, QuantumShield Gov 92, GovWatch Behavioral 85 |

Totaux : 2 160 ¢ de compromissions, 8 250 ¢ de fichiers (10 fichiers), 410 XP de compromissions. Valeurs `firewall` (2, 5, 8, 4, 6, 7, 9) : affichées par `traceroute`, abaissées par `uploadvirus`, **utilisées par aucune formule** `[C]`.

État persistant par système : découvert, compromis, backdoor, virus, tracé, accès internes (`has_intel`), fichiers extraits. Un système compromis ne paie plus. Les **fichiers ne se lisent pas** : `content` n'est affiché nulle part `[C]` (grep `.content` dans `src/`) ; ils ne servent que de réserve de crédits. Le niveau de chiffrement des fichiers sert de seuil pour la clé (≤ 2) et pour l'extraction profonde, rien d'autre : `quantumdecrypt` n'ouvre aucun fichier.

### 1.4 Formules de réussite

**Méthodes classiques** (`world.c:207-234`) : `chance = clamp(5, 95, base + par_niveau×N − par_sécurité×S − malus_critique + bonus + 15 si intel)`.

| Méthode | base | / niveau | / sécurité | malus critique | bonus ajoutés par la commande |
|---|---|---|---|---|---|
| BRUTE | 80 | 0 | 15 | 0 | − pénalité d'alerte |
| BACKDOOR | 70 | +10 | 15 | 0 | +20 si `stealth_mode` ; − pénalité |
| VIRUS | 60 | 0 | 10 | 0 | + furtivité du virus ; +30 si backdoor posée ; − pénalité |
| AI | 85 | +5 | 0 | 20 | − pénalité |
| EXPLOIT | 60 | +5 | 12 | 0 | − pénalité |

Pénalité d'alerte (`alert.c:81-90`) : −5 dès 30, −10 dès 50, −20 dès 80 ; la bande « DANGER » (70) n'ajoute **aucun** cran alors que son message promet des hacks « nettement moins sûrs ». Elle ne s'applique ni à `socialeng` ni à `temporalhack`.

Valeurs `[E]` (sonde `nh_world_chance`) :

| | sécu 1 | sécu 2 | sécu 3 | sécu 4 (critique) |
|---|---|---|---|---|
| BRUTE, par essai (niveau sans effet) | 65 | 50 | 35 | 20 |
| **BRUTE, par commande (5 essais)** | **99,5** | **96,9** | **88,4** | **67,2** |
| BACKDOOR niveau 1 → 6 | 65 → 95 | 50 → 95 | 35 → 85 | 20 → 70 |
| VIRUS (hors bonus) | 50 | 40 | 30 | 20 |
| AI niveau 1 → 6 | 90 → 95 | 90 → 95 | 90 → 95 | 70 → 95 |
| EXPLOIT niveau 1 → 6 | 53 → 78 | 41 → 66 | 29 → 54 | 17 → 42 |

**Méthodes avancées** (`advanced_hacking.c:492-548`, 8 méthodes) :
`taux = clamp(1, 99, base_méthode + 5×niveau − sécurité_cible/2 + XP/100 + puissance_outil/10 + 5 + 10) + 15 si intel − pénalité d'alerte`
(+15 « mode fantôme » et +20 « intrication quantique » : jamais actifs ; malus de défenses « adaptatives » : jamais, `attack_count` ne s'incrémente nulle part `[C]`). Le « +5 » est `upgrade_level` (constant 1), le « +10 » l'assistant IA « loyal » (constant). Au niveau 5 contre nexus : Zero-Day 99, Neural 94, Quantum 89, Social 83, AI 77, Virus 72, Stealth 66, Ghost 50 ; au niveau 3 : 87 / 82 / 77 / 71 / 65 / 60 / 54 / 38 `[E]`. Succès : aucune alerte, batterie de l'outil −`energy_cost`, compromission profonde. Échec : gravité = (risque de détection + 100 − taux)/20 → ≥ 5 : alerte +30 ; ≥ 3 : +15 et sécurité de la cible +1 ; ≥ 1 : +5 (`advanced_hacking.c:821-848`).

**Ingénierie sociale** (`advanced_hacking.c:623-633`) : `50 + 5×niveau + 2×réputation − sécurité/2`, plafonné 5-95. La réputation n'a pas de plafond de jeu (jusqu'à 1 000 000) : dès ≈ 40 points (la 3e quête en demande 50), 95 % sur tous les systèmes. **Temporel** : `30 + 5×niveau + 0` (`processing_power` n'est jamais renseigné).

### 1.5 Alerte (une jauge 0-100, `alert.c`, `alert.h:19-23`)

| Seuil | Effet |
|---|---|
| 30 | bande « ATTENTION » (jaune), pénalité −5 |
| 50 | message « niveau d'alerte élevé », pénalité −10 |
| 70 | bande « DANGER » (rouge), aucun malus supplémentaire |
| 80 | boutique fermée, message « critique », pénalité −20 |
| 100 | **game over** (la partie fatale n'est pas sauvegardée : « Continuer » reprend avant la commande fatale) |

Refroidissement : −1 par commande HACK/ADVANCED, −2 avec le VPN (60 ¢, permanent). Proxy Chain : les hausses sont réduites d'un tiers (arrondi au-dessus) pendant 5 hacks (max 15), décompte aussi par les commandes sans effet. `laylow` (`alert.c:93-101`) :

| N° | Méthode | Prix | Baisse | Remarque |
|---|---|---|---|---|
| 1 | Attendre | 0 | −8 | **gratuit et illimité** |
| 2 | VPN | 20 | −10 | |
| 3 | Proxies | 30 | −15 | |
| 4 | Ghost Protocol | 0 + 1 charge (objet 80 ¢, max 5) | −30 | dominé par le n° 5 |
| 5 | Se faire discret | 40 | −25 | |
| 6 | Faire accuser un autre | 60 | −35 | −5 de réputation |

Aucune de ces actions ne fait « passer le temps » : elles ne comptent ni pour le refroidissement ni pour les proxies. Mesure `[E]` : à alerte 60, cinq `stealthmode` → 55, cinq `analyzedefenses` → 50, un `laylow` 1 → 42, crédits intacts.

### 1.6 Boutique (`shop.c:20-37`, effets `shop.c:100-159`)

| Objet | Prix | Niv. | Type | Effet réel | Plafond (achat refusé au-delà) |
|---|---|---|---|---|---|
| Stealth Module v2.0 | 150 | 2 | unique | +2 furtivité (ouvre `stealth` s'il atteint 5) | furtivité 10 |
| Ghost Protocol | 80 | 1 | conso. | +1 charge pour `laylow` n° 4 (−30) | 5 |
| Malware Arsenal | 120 | 4 | unique | bibliothèque de 3 virus | 3 |
| Proxy Chain Pro | 100 | 2 | conso. | +5 hacks à hausses d'alerte −1/3 | 15 |
| Quantum Encryption Key | 200 | 3 | unique | extrait les fichiers de chiffrement ≤ 2, y compris sur les systèmes déjà compromis | — |
| Neural Assistant v3.1 | 500 | 4 | unique | l'IA : `aihack` (avec le niveau 5), `aiassist` | — |
| Quantum Processing Chip | 800 | 5 | unique | `quantumdecrypt`, outil quantique | — |
| Street Cred Booster | 75 | 1 | conso. | **+20 de réputation, sans plafond de stock** | aucun |
| Neural Accelerator | 90 | 2 | conso. | doubles 3 gains d'XP (≤ +30 chacun) | 3 charges |
| Dark Web VPN | 60 | 1 | unique | refroidissement +1 par hack | — |

Total, un de chaque : 2 175 ¢ (consommables : 345 ¢, rachetables). R4Z0R n'est **pas** requis pour acheter : `shop` est ouvert dès le niveau 1.

### 1.7 Récompenses (toutes sources)

| Source | XP | ¢ | Autre | Borne |
|---|---|---|---|---|
| scans 1 à 5 | 15 au total | — | alerte +1 chacun | `scans_done` |
| compromission (1re fois) | 410 (7 systèmes) | 2 160 | fichiers si profonde ou clé | drapeau du nœud |
| fichiers | — | 8 250 (10 fichiers) | | `is_unlocked` |
| traceroute | 8 × 7 = 56 | — | | `is_traced` |
| backdoor | 15 × 7 = 105 | **500 × 7 = 3 500** | | `has_backdoor` |
| uploadvirus | 20 × 7 = 140 | 250 / 400 / 700 selon le virus tiré (≈ 3 000) | | `has_virus` |
| socialeng | 20 × 5 = 100 | — | +15 de chance sur le système | `has_intel` |
| decrypt de test | 20 | — | jalon | `milestones` |
| quantumdecrypt | 50 | 2 000 | jalon | `milestones` |
| quantumdecrypt `CLASSIFIED` | — | **10 000** | jalon | `milestones` |
| quêtes (3 écrites) | 25, 40, 80 (145) | 200, 300, 1 000 | réputation 25, 40, 100 | statut |
| tutoriel | 0 | 100 | réputation 10 | statut |
| **Total** | **1 041** (le niveau 6 en demande 420) | **≈ 30 700** (+ 100 au départ) | | |

La réputation vient des quêtes (175 au total) et du Street Cred Booster ; elle baisse de 5 avec « Faire accuser ». Plafonds de sécurité : XP 1 000 000, réputation ±1 000 000, crédits 10⁹ à la lecture d'une sauvegarde.

### 1.8 Contacts (`contacts.c:66-179`)

| Contact | Déblocage (toutes conditions) | Sujets | Effet de jeu |
|---|---|---|---|
| ECHO-7 | départ | 6 (conseil, progression, mission, Nexus, dossier, fin) | répète la consigne du tutoriel |
| R4Z0R | niveau 2, réputation ≥ 10 | 6 (boutique, équipement, argent, nouvelles, dossier, fin) | ouvre `shop` |
| Phoenix | niveau 4, réputation ≥ 50, « Réseaux d'Information » terminée | 5 | donne « L'œil du Cyclone » |
| AURA | niveau 5, réputation ≥ 50, « L'œil du Cyclone » terminée | 5 | aucun |
| Shadow Broker, Neon Angel, Ghost Walker, Data Miner, Nexus Insider | **jamais** (pas de fiche : `topic_count == 0`) | — | — |

Confiance déduite : départ 60 / 30 / 0 / 0 + 3 par conversation, relation inconnu < 20 ≤ neutre < 50 ≤ amical < 80 ≤ de confiance : **affichage seulement**. Courrier : 4 modèles (bienvenue, R4Z0R, Phoenix, AURA). Les répliques sont statiques.

### 1.9 Quêtes (`quest_system.c:29-112`, dix emplacements, `quest_system.h:34-47`)

| Quête | Contact | Niv. | Prérequis | Objectifs | Récompense |
|---|---|---|---|---|---|
| 1 Premiers Pas dans l'Ombre (INTRO_TUTORIAL) | ECHO-7 | 1 | — | manuel (7 étapes du tutoriel : `quests`, `help`, `scan`, `status`, niveau 2, `bruteforce localhost`, `laylow`) | 100 ¢, rép. 10 |
| 2 Baptême du Feu (FIRST_INFILTRATION) | ECHO-7 | 2 | 1 | compromettre corp-server-01 ; alerte < 50 *à l'instant de conclure* | 25 XP, 200 ¢, rép. 25 |
| 3 Réseaux d'Information (GATHER_INTEL) | R4Z0R | 3 | 2 | parler à R4Z0R ; acheter le Stealth Module ; 3 systèmes compromis ; réputation ≥ 50 ; décrypter (secret) | 40 XP, 300 ¢, rép. 40 |
| 4 L'œil du Cyclone (NEXUS_DATA_BREACH) | Phoenix | 4 | 3 | acheter clé, IA ou puce ; compromettre nexus-mainframe ; ≥ 1 fichier extrait de nexus | 80 XP, 1 000 ¢, rép. 100 |
| 5 à 10 : UNDERGROUND_CONTACT, CORPORATE_SABOTAGE, AI_LIBERATION, SHADOW_BROKER, FINAL_SHOWDOWN, EPILOGUE | — | — | — | **aucune définition** (`objective_count == 0`, elles restent verrouillées) | — |

Les quêtes sont numérotées de 1 à 10 dans ce document (l'enum `QuestType` du code commence à 0).

Neuf types d'objectifs, mesurés sur l'état du jeu (`quest_system.c:202-237`) : MANUAL, REACH_LEVEL, BUILD_REPUTATION, HACK_TARGET, GATHER_DATA, MEET_CONTACT, PURCHASE_ITEM, DECRYPT_MESSAGE, KEEP_ALERT_BELOW. Au plus 5 objectifs et 2 prérequis par quête ; 4 chapitres annoncés (L'éveil du hacker, Dans l'ombre des corporations, Le Projet Aurora, La libération). Les statuts DISPONIBLE et ÉCHOUÉE ne sont jamais produits.

### 1.10 Hacking avancé : l'état réel

8 méthodes (succès / détection / énergie / outil requis, `advanced_hacking.c:20-111`) : Quantum 85/15/40 (puce), AI-assisted 75/25/30 (IA), Neural 90/35/50 (sync ≥ 50), Stealth 65/5/25 (`stealth_mode`), Virus 70/45/35 (bibliothèque > 0), Social 80/20/20 (intel quelque part), Zero-Day 95/60/60 (`exploit` débloqué), Ghost 50/0/10 (charges Ghost > 0). Chaque outil a 100 % de batterie, **ne se recharge jamais** (le texte du menu annonce `tools`, `activate`, `upgrade`, `recharge` : ces commandes n'existent pas) : capacité totale 29 succès pour 5 cibles, donc sans effet de jeu aujourd'hui mais piège silencieux. `power_level`, `upgrade_level`, `player_hacking_level`, `god_mode_unlocked`, `detection_meter`, `quantum_cores`, `temporal_hack_unlocked`, `SecuritySystem` : écrits ou affichés, jamais lus par une règle `[C]`. `display_defense_analysis` affiche « 95/10 » pour une note sur 100 (`advanced_hacking.c:601-603`).

### 1.11 Divers

- **Aléa** : `rand()` de la libc (`cmd_hacking.c`, `advanced_hacking.c`), `srand` à l'heure ou à `--seed` (`main.c:50`). Le PCG32 de `src/core/rng.c` existe et est testé mais **n'est appelé par aucune règle de jeu** `[E]` (grep). `--seed` n'est donc reproductible que sur une même libc.
- **Animations** : `print_typing_effect(texte, ms)` attend `ms` **par caractère**. Mesuré avec animations actives `[E]` : backdoor 38 s, uploadvirus 33 s, aihack 34 s, quantumdecrypt 48 s, exploit 24 s, traceroute 7,5 s, scan 1,5 s, bruteforce 0,9 s par essai (4,5 s au plus), prologue 8,2 s. Pas de saut par touche ; seul le réglage global `--fast`/« animations » les coupe.
- **Sauvegarde** : texte `clé=valeur` versionné (`neon-hack-save=1`), chargement transactionnel, écriture atomique, **un seul emplacement**, sauvegarde automatique après chaque commande sauf partie perdue (`save.c`, `game.c:123-160`).
- **Textes** : 436 clés dans `strings.def` (≈ 20 000 caractères français ; ≈ 13 000 de narration, soit ≈ 2 200 mots) ; environ 570 lignes d'affichage en français restent codées en dur dans les modules non portés (ROADMAP).
- **Tests** : 26 suites unitaires passent dans une copie `[E]` (`make unit`) ; les 105 vérifications e2e et 75 pty de l'énoncé n'ont pas été rejouées ici.

---

## 2. Critique

### 2.1 À garder en esprit

- **Le ton et le décor** : Neo-Tokyo 2087, Nexus Corp et le Projet Aurora, ECHO-7 (la voix qui forme), R4Z0R (le marché), Phoenix (l'ex-initié traqué), AURA (l'IA qui a pris conscience). Les 4 chapitres (L'éveil du hacker, Dans l'ombre des corporations, Le Projet Aurora, La libération) forment déjà une charpente en actes.
- **L'identité « ligne de commande »** : aide générée depuis la table de commandes, commandes révélées par le niveau, complétion TAB qui ne propose jamais ce qui est verrouillé.
- **Une seule source de vérité en tables**, des quêtes « à niveau » qui relisent l'état du jeu (robustes à l'ordre du joueur et aux anciennes sauvegardes), des annonces livrées *après* le résultat de la commande (bus différé).
- **Le graphe de relais** (il faut ouvrir la route) : c'est la graine d'une carte tactique ; on l'approfondit au lieu de le jeter.
- **Une jauge 0-100 aux bandes nommées** (30 / 50 / 70 / 80 / 100) qui annonce ses conséquences.
- **Le tutoriel comme première mission** dans la vraie boucle (étapes lues sur l'état, récompense versée une fois, passable).
- **La règle anti-farm** (toute récompense bornée par un état) et **l'unique point d'entrée de l'XP**.
- **Sauvegarde versionnée, transactionnelle, atomique**, et la culture de tests d'invariants (section 4).

### 2.2 Impasses, redondances, code mort

| # | Constat | Preuve | Gravité |
|---|---|---|---|
| D1 | **Deux états de furtivité** : `stealth` (caché, sauvegardé, +20 backdoor) et `stealthmode` (visible, ne bascule rien) | `cmd_hacking.c:401-425`, `cmd_advanced.c:113-131` ; sonde `[E]` | haute |
| D2 | Le Stealth Module (150 ¢) n'ouvre qu'une commande cachée (+20 sur `backdoor`) ; la quête 2 l'impose | `shop.c:105-110`, `quest_system.c:80` | moyenne |
| D3 | Objets dominés : Ghost Protocol (80 ¢ pour −30) < « Se faire discret » (40 ¢ pour −25) ; « Attendre » (gratuit, illimité) bat en crédits tout `laylow` payant, qui ne fait gagner que des commandes ; Proxy Chain Pro et VPN marginaux ; Malware Arsenal donne au niveau 4 ce que le niveau 5 donne gratuitement | `alert.c:93-101`, `progression.c:100-108` | haute |
| D4 | **Réputation achetable sans limite** (Street Cred Booster) : elle ne filtre plus rien, et `socialeng` (`2×réputation`) devient certain | `shop.c:81-82` | haute |
| D5 | `aiassist`, `neuralsync` (hors seuil 50 / 90), `stealthmode`, `analyzedefenses` : sans effet ; refroidissent l'alerte gratuitement | 1.1 ; sonde `[E]` | haute |
| D6 | Mécaniques annoncées et absentes : coût de furtivité « par minute », commandes `tools`/`activate`/`upgrade`/`recharge`, défenses « adaptatives », mode fantôme, intrication quantique, mode Dieu | 1.10 | moyenne |
| D7 | `firewall_strength` cosmétique ; `exploit` dominé (une seule tentative à 17-78 %) alors que `bruteforce` fait 5 essais | 1.3, 1.4 | moyenne |
| D8 | **Le contenu des fichiers n'est jamais montré** : aucun canal de lore ; `CLASSIFIED` (12 000 ¢) est un mot de passe introuvable en jeu ; `decrypt` exige de recopier un chiffré qu'il faut aller chercher dans le message d'usage | `world.c:44-69`, `cmd_hacking.c:140,515` | haute |
| D9 | Le virus est tiré au hasard : le joueur ne choisit rien ; `is_detected` est écrit, jamais lu | `cmd_hacking.c:357` | basse |
| D10 | Récompenses extrêmes : +500 ¢ par backdoor (plus que 5 systèmes sur 7 ne paient à leur compromission), 12 000 ¢ de `quantumdecrypt` | `cmd_hacking.c:221,529,564` | haute |
| D11 | Cinq contacts sur neuf et six quêtes sur dix inexistants ; statuts quête DISPONIBLE/ÉCHOUÉE jamais produits | 1.8, 1.9 | bloquante (campagne) |
| D12 | Aléa non reproductible entre plates-formes, alors que le PCG32 est écrit et inutilisé | 1.11 | moyenne |
| D13 | Affichages faux : « 95/10 », « Temps : %ds » (ce sont des ms par bloc), `time_required` | `advanced_hacking.c:374,601` | basse |
| D14 | Incohérences de texte : « Agent Smith » (placeholder), « Projet Ghost Protocol » dans le document alors que le projet s'appelle Aurora, AURA se dit déjà « libérée » au niveau 5 alors qu'une quête « AI_LIBERATION » est prévue plus tard | `cmd_hacking.c:521-524`, `quest_system.h:42` | moyenne |

### 2.3 Profondeur : est-ce « bruteforce jusqu'à ce que ça passe » ? `[E]`

Bot « focused » : `scan` quand il y a du nouveau, puis `bruteforce` sur chaque système atteignable, `laylow` 1 selon une politique d'alerte ; tutoriel joué ; 500 graines.

| Politique d'alerte | Game over | Commandes de hacking / échecs par partie | Attentes `laylow` | Commandes jusqu'au niveau 6 |
|---|---|---|---|---|
| prudente (attendre dès 30 jusqu'à < 15) | 0 % | 11,0 / 1,0 | 12,0 | 30,2 (26-55) |
| équilibrée (dès 50 jusqu'à < 30) | 0 % | 11,1 / 1,1 | 8,2 | 28,9 (24-53) |
| hardie (dès 70 jusqu'à < 50) | 0 % | 11,7 / 1,7 | 8,8 | 29,1 (20-60) |
| aucune | **75 %** (375/500) | 10,7 / 1,5 | 0 | 20,4 |

Lecture : sur ≈ 11 commandes d'intrusion, ≈ 1 échoue. La seule décision est « attendre ou pas », et attendre est gratuit. Les autres méthodes n'offrent pas un autre jeu :

- `backdoor`, `uploadvirus` : jouées pour leur prime (500 ¢ ; 250-700 ¢) et l'XP unique, pas pour leur effet.
- `exploit` : une tentative contre cinq pour `bruteforce` ; sert uniquement à sauter un relais, ce que le graphe ne rend jamais nécessaire.
- `aihack` (niveau 5 + 500 ¢) : 90-95 % partout, extrait **tous** les fichiers, alerte 5 : domine tout, et la clé de chiffrement (200 ¢) n'a plus d'objet.
- `advhack` : taux 38-99 %, mais le niveau et l'XP (`XP/100`) montent les taux sans choix, et le meilleur outil (Zero-Day, 1 usage) est non rechargeable.
- Aucune information cachée à gagner, aucun compromis (bruit contre vitesse, outil contre cible) : la sécurité d'un système ne se « lit » que comme un nombre.

### 2.4 Rythme `[E]`

Bot « complétiste » (tutoriel, boutique, `contact R4Z0R`, `decrypt`, toutes les sources uniques, `aihack` dès que possible), 500 graines par politique :

| Jalon | Commandes cumulées |
|---|---|
| fin du tutoriel (niveau 2) | 8-10 |
| niveau 3 / 4 / 5 / 6 | 15 / 19 / 21 / ≈ 33 |
| « Baptême du Feu » / « Réseaux d'Information » terminées | 15 / 19 |
| 7 systèmes compromis | ≈ 33 (donc bien avant la fin de « L'œil du Cyclone », à ≈ 94 : son objectif de brèche est rempli très tôt) |
| **tout le contenu actuel épuisé** | **98** (hack : 42, attentes : 37), crédits finaux ≈ 29 000 |
| attente d'animations cumulée (animations actives, TTY ; moyenne sur 200 graines, politique équilibrée) | **790 s** (13 min) pour le complétiste, 31 s pour le joueur « focused » |

Le niveau 6 arrive au tiers du contenu : l'axe de progression est épuisé avant que l'histoire ait commencé (quêtes 5 à 10 absentes).

### 2.5 Économie

Revenus uniques ≈ 30 700 ¢ (1.7) contre 2 175 ¢ de boutique et 20 à 60 ¢ par `laylow` payant. Le complétiste finit à ≈ 29 000 ¢ sans rien à acheter. Le niveau 5 est le point de rupture : la puce quantique (800 ¢) se « rembourse » avec 12 000 ¢ à la première commande. Deux effets corollaires : la réputation s'achète (D4), et aucune décision d'achat n'existe après la mi-partie.

### 2.6 Alerte

L'alerte ne produit ni choix ni tension : (1) gratuité de « Attendre » (1.5, D3) ; (2) bande DANGER (70) sans effet propre ; (3) game over sans conséquence narrative (rechargement de la sauvegarde précédente, un seul emplacement) ; (4) pénalités qui ne s'appliquent pas à toutes les méthodes. À garder : les bandes nommées et annoncées. À changer : le fait qu'elle soit une ressource globale remise à zéro par la patience.

### 2.7 Un monde de 7 systèmes peut-il porter une campagne complète ? Non.

| Besoin d'une campagne complète (10 quêtes, 9 contacts, épilogue) | Offre actuelle | Écart |
|---|---|---|
| ≥ 10 quêtes, chacune avec au moins un lieu d'action propre | 7 systèmes, dont au moins 3 déjà compromis quand s'ouvre la 4e quête (la 3e en exige 3) | il manque ≥ 8 lieux |
| une progression qui dure autant que l'histoire | 6 niveaux, le dernier atteint vers la 30e commande | axe à refondre |
| 9 contacts qui ont chacun un rôle | 4 fiches, 1 seul avec effet de jeu (R4Z0R) | 5 fiches + 8 rôles |
| de la matière narrative | 10 fichiers dont rien n'est lu, 4 courriers | canal de lore à créer |
| un épilogue qui dépend de ce qu'on a fait | `alert.max_level` pour seul souvenir | drapeaux de choix et statistiques |
| un climax | aucun boss : le final serait `bruteforce gov-database` (67,2 %) | mécanique de rencontre |
| des quêtes qui ne soient pas des listes de courses | 9 types d'objectifs mesurables, aucun choix, aucune scène | langage de missions |

Propriété aggravante du moteur : une quête dont l'objectif est déjà vrai à l'ouverture se termine d'un coup (`ARCHITECTURE.md`, « Objectifs à niveau »). Avec 7 lieux fixes, les quêtes 5 à 9 de type « pirater X » s'enchaîneraient instantanément pour tout joueur qui explore librement `[H]` (conséquence du mécanisme décrit, non simulée : ces quêtes n'existent pas).

### 2.8 Ce qui bloquerait une campagne complète

- **B1 Contenu absent** : quêtes 5-10, 5 contacts, épilogue, textes (≈ 2 200 mots narratifs aujourd'hui).
- **B2 Lieux** : 7 systèmes sans gating d'histoire ; il faut des sites déverrouillés par des événements, pas seulement par `scan` et le niveau.
- **B3 Progression épuisée** au tiers (niveau 6) ; les niveaux viennent des nœuds, pas des quêtes (145 XP de quêtes sur 1 041).
- **B4 Vocabulaire de missions trop pauvre** (pas de drapeaux, choix, scènes, rencontres, états d'échec).
- **B5 Pas de canal de lore** (fichiers muets, pas de documents, cinématiques retirées).
- **B6 Pas d'embranchements ni de bilan** : rien pour calculer l'épilogue.
- **B7 Échec = game over** sans habillage, sauvegarde à un emplacement : une longue campagne y perd tout retour arrière.
- **B8 Économie** vide de sens après le niveau 5 ; réputation achetable.
- **B9 Rythme du texte** : des attentes jusqu'à 48 s par commande, non interruptibles, multipliées par des centaines de commandes.
- **B10 Couplage au code** : `NhNode`, `ContactType`, `QuestType`, indices de tableaux écrits dans les sauvegardes (« ne pas réordonner ») ; ajouter un lieu ou un contact demande du code et fige le format. Or l'histoire doit pouvoir évoluer.

### 2.9 Constats pour la TUI et l'accessibilité

- Le HUD ANSI par région de défilement **ne permet pas de panneau à droite** (`ARCHITECTURE.md`, « Limite connue ») : la TUI doit posséder l'historique du texte.
- La logique de règle et la mise en scène sont mêlées : les pauses vivent dans les handlers (`cmd_hacking.c`). Le moteur Rust doit renvoyer des événements et laisser la vitesse au frontend.
- Des contraintes qui excluent des joueurs : recopier des chaînes (`decrypt WKLV#LV#D#WHVW`), deviner un mot secret, attentes imposées, émojis dans le code non porté (déjà interdits dans le code porté, test `test_no_emoji_anywhere`), couleur comme seul signal (jauge rouge/jaune/vert sans libellé partout).
- Ce qui aide déjà : tout est texte, pas de temps réel, `NO_COLOR`, `--fast`, saisie avec complétion, anglais/français.

---

## 3. Redesign

### 3.1 Principes (issus de l'audit)

- **P1 Prévisible** : une décision change un résultat que le joueur peut anticiper ; la prévision et la résolution sont **la même fonction**.
- **P2 Le hasard sert la variété, pas la décision** : butin, ordre de patrouille annoncé, variantes de texte ; jamais « 65 % de réussir ».
- **P3 Pas de XP par action répétable** : la progression vient des missions ; l'anti-farm devient structurel (registre de gains uniques).
- **P4 L'histoire est de la donnée** : missions, scènes, dialogues, documents, conditions et effets en données validées par tests ; le moteur ne connaît aucune intrigue.
- **P5 Échouer a un coût narratif, jamais une impasse** ; le game over n'existe qu'en mode Hardcore.
- **P6 Une seule vue pour deux frontends** : le moteur produit des événements structurés (clé de texte + arguments typés) ; le frontend plain est la référence ; la TUI ne montre rien de plus que ce que le texte dit.
- **P7 Aucun temps réel, aucune attente dans le moteur** ; la cadence du texte est un réglage de présentation.

### 3.2 Trois directions

| | **A — Infiltration à tours** | **B — Contrats à jets lisibles** | **C — Terminal vivant** |
|---|---|---|---|
| Idée | un « run » = intrusion tour par tour dans un petit graphe d'ICE visibles ; jauge de Trace | une mission = 3-5 phases, une approche par phase, chance affichée avec le détail de ses modificateurs | exploration d'un faux système (`ls`, `cd`, `cat`, `connect`, `probe`, `breach`), mots de passe trouvés dans les fichiers |
| Hasard | quasi nul (butin, patrouilles annoncées) | un jet par phase, « sac de tirages » sans séries, relance en dépensant du Focus | nul (énigmes) |
| Décisions typiques | quel programme sur quel ICE, quand masquer, quand sortir | quelle approche, quoi risquer, quand relancer | quoi lire, où chercher, quoi déchiffrer |
| Lore | documents de butin, scènes de mission | scènes de phase | fichiers et journaux (très riche) |
| Moteur Rust `[H]` | 9,5-15 k lignes | 6-9 k | 8-12 k |
| Texte FR `[H]` | ≈ 29 k mots | ≈ 25 k | ≈ 35 k |
| Par mission, à écrire | graphe de 4-8 nœuds + ICE + butin + scènes | 3-5 phases + modificateurs + scènes | 6-15 hôtes avec fichiers, journaux, énigmes |
| Preuve de complétude (pas d'impasse) | **solveur exact** par mission (spike : ms) | arbre fini, trivial | difficile : solveur d'énigmes + indices |
| Ajustement de difficulté | marge de Trace calculée par le solveur | % affichés | indices et verrous |
| TUI | excellente (carte, Trace, deck) | moyenne (fiche de dossier) | bonne (arbre de fichiers) |
| Plain / lecteur d'écran | très bon (état textuel court à chaque tour) | très bon | bon, sauf les recopies de codes |
| Risque principal | équilibrage et ergonomie des runs ; plus gros chantier | lassitude des menus, fantasme « hacker » faible | blocages d'énigmes, frappes de navigation (`cd`/`ls`), contenu massif |
| Fit priorités (1 campagne, 2 TUI, 3 accessibilité) `[H]` | 4 / 5 / 5 | 5 / 2 / 5 | 3 / 3 / 4 |

**Recommandation : A, avec un chemin de repli intégré.** B est le moins risqué pour la priorité 1, mais c'est « les dés en plus lisibles » : il garde le défaut central de l'existant (choisir le meilleur pourcentage) et n'apporte presque rien à la TUI. C est séduisant mais le contenu (≈ 35 k mots + énigmes sans impasse) menace la complétude. A offre la meilleure TUI et la preuve de complétude par solveur ; son risque (le gros chantier) est réduit en ne livrant d'abord qu'un **noyau** (3.3) et en mettant les runs derrière une interface à deux implémentations : `AutoResolve` (mode histoire, aussi utilisé comme bouchon) et `Tactical`. La campagne complète se joue donc de bout en bout avant que la tactique soit finie.

### 3.3 Direction A : les règles

**Boucle** : `Planque (hub)` → `briefing` (contact, choix) → `mission` (1 à 3 scènes et runs) → `débriefing` (documents lus, conséquences, récompenses) → `Planque`.
Commandes du hub : `missions`, `talk <contact>`, `shop`, `deck`, `equip`, `laylow` (services de Notoriété, nom conservé), `messages`, `journal`, `net` (carte des sites), `status`, `hint`. Commandes d'un run : `probe <n>`, `breach <n> <programme>`, `move <n>`, `loot`, `cloak`, `spoof`, `end`, `jackout`, `undo`, `map`, `status`.

**Le run** (valeurs de départ, à régler au solveur et au simulateur) :
- **Graphe** de 4 à 8 nœuds (passerelle, pare-feu, coffre, capteur…), arêtes visibles après `probe`. Chaque ICE a une **famille** (Réseau, Chiffrement, Humain, Gardien IA) et une **force** de 1 à 6.
- **Cycles** : 3 par tour (4 au palier 3, 5 au palier 5). Chaque action coûte des cycles ; `end` termine le tour.
- **`breach`** : ajoute la **puissance** du programme à une **progression qui persiste** (2/3, puis 3/3) ; pas de dé. Un programme a une puissance, un coût en cycles, un **bruit** (Trace ajoutée), des charges. **Règle de famille** : contre un ICE d'une autre famille que le sien, un programme n'apporte que 1 de puissance (sauf mention) ; c'est ce qui rend utiles `probe` et le choix du deck.
- **Trace** 0-100, remise à zéro à chaque run : bruit des actions + ambiante (+2 par tour : attendre n'est jamais gratuit, donc tout run se termine) + scans des **Sentinelles** non neutralisées (+5 par Sentinelle voisine). Bandes conservées de l'existant : 30 vigilance, 50 alerte (les Sentinelles voient à distance 2), 70 danger (+2 de bruit par brèche), 100 **grillé** (échec du run).
- **Prévision** : à chaque tour, une ligne annonce ce qui va se passer à la fin du tour (« Sentinelle de Pare-feu scanne Archives : +5 Trace, sauf si masqué »). L'aléa n'entre pas dans cette ligne.
- **Programmes** (remplacent les 8 « outils », 8 méthodes avancées, 5 méthodes classiques et le virus au hasard) :

| Programme | Famille | Puissance | Cycles | Bruit | Charges / run | Origine (ancien) |
|---|---|---|---|---|---|---|
| Brute-Force | Réseau | 2 | 1 | 5 | illimité | `bruteforce` |
| Exploit | Réseau | 3 | 2 | 2 | 2 | `exploit`, Zero-Day |
| Backdoor | Réseau | 1 | 1 | 1 | illimité ; le prochain run sur ce site démarre depuis ce nœud | `backdoor` |
| Décryptage | Chiffrement | 2 | 1 | 3 | illimité | `decrypt`, clé de chiffrement |
| Quantique | Chiffrement | 4 (1 hors famille) | 2 | 2 | 2 | puce quantique |
| Ingénierie sociale | Humain | 3 | 1 | 1 | 2 ; exige une information (Intel) du site | `socialeng` |
| Virus | Gardien IA | 2 + neutralise une Sentinelle 2 tours | 2 | 4 | 1 | `uploadvirus`, Malware Arsenal (3 variantes **choisies**) |
| Spoof | utilitaire | −10 Trace | 1 | 0 | 2 | Proxy Chain, VPN |
| Cloak | utilitaire | annule le prochain scan | 1 | 0 | 2 | Stealth Module, `stealth` |
| Ghost Protocol | utilitaire | annule tout le bruit du tour | 1 | 0 | 1 (consommable) | objet Ghost |
| Overclock | utilitaire | +2 cycles ce tour | 0 | 3 | 1 | Neural Accelerator, `neuralsync` |
| Compagnon (ECHO, puis AURA) | compagnon | 1 `probe` gratuit par tour | — | — | — | Neural Assistant, `aiassist`, `aihack` |

- **Fin d'un run** : `jackout` avec le butin, ou **grillé** à 100 : le butin non extrait est perdu, la Notoriété monte, le run se rejoue (mode par défaut) ; seul Hardcore ferme la partie.
- **Annuler** : `undo` rend le dernier tour (le moteur est pur et l'état cloneable) ; réglable (3.8).
- **Aléa conservé** : variante de butin, ordre des patrouilles (post-1.0), graine dans l'état de partie (PCG32 déjà écrit dans `src/core/rng.c`).

**Preuve par le solveur (spike `[E]`)**, mission jouet de 6 nœuds (Brute + Exploit, 2 Cloak, 3 cycles, scan 5, ambiante 2) : 8 393 états, **3 à 5 ms**, plan optimal en 5 fins de tour, Trace finale 24/100 ; recherche exhaustive indépendante : 3 071 546 états, 3,2 s, **même résultat**. Avec Brute seul : Trace 28 (1 137 états, 0,4 ms) ; mode histoire (4 cycles, scan 2) : 17 (9 870 états, 4 ms) ; règles « expert » (scan 9, Trace max 40) : toujours 24 car le plan optimal évite les scans. Enseignements : (1) chaque mission peut être **déclarée solvable avec la panoplie minimale du palier** par un test ; (2) la difficulté se règle sur la **marge** (Trace max − Trace minimale du solveur), pas à l'instinct ; (3) le même solveur permet de détecter les programmes dominés (retirer un programme n'allonge aucun plan ⇒ à rééquilibrer) `[H]`. Limite : jouet sans patrouilles ni familles ; avec elles, l'espace grossit (borne à surveiller, § 5).

**Exemple de tour (frontend plain ; tour 3 du plan optimal du spike, habillé avec les noms du jeu ; le jouet ignore les familles, tous les ICE franchis y sont de la famille Réseau)** :

```
RUN  MegaCorp Industries · serveur de paie        Tour 3   Trace 8/100 (calme)   Cycles 3/3
Nœuds   P passerelle · F pare-feu (ouvert) · A archives (ouvert) · C coffre (Réseau 4, 0/4)
        S sentinelle (Gardien IA 2) · K caméras (Gardien IA 3)
Liens   P-F  P-S  F-A  S-A  A-C  A-K  C-K
Prévision : les Sentinelles S et K scrutent A à la fin du tour (+5 Trace chacune) sauf si A est masqué.
> move A
Vous êtes en A.
> cloak
Cloak actif : le prochain scan est annulé (reste 1).
> breach C brute
Brute-Force (puissance 2) sur Coffre : progression 2/4. Trace +5 (13/100).
> end
Fin du tour. Trace ambiante +2 (15/100). Scans de S et K annulés.
```

**Maquette TUI** (même contenu, rien de plus ; la carte du panneau est une version abrégée de la liste de liens du plain) :

```
┌ Journal ────────────────────────────────────────┐┌ Trace ─────────────────┐
│ Tour 3 · MegaCorp Industries                    ││ ▓▓▓▓░░░░░░  15/100     │
│ Archives : en place. Cloak actif (reste 1).     ││ Cycles  ● ● ○          │
│ Prévision : S et K scannent A, annulé (Cloak).  ││ Notoriété : Discret    │
│                                                 │├ Carte ─────────────────┤
│ > breach C brute                                ││ P─F✓─A─C 2/4           │
│ Brute-Force : Coffre 2/4. Trace +5.             ││ S!──┘   K!             │
│ > _                                             │├ Deck ──────────────────┤
│                                                 ││ 1 Brute  2 Exploit (1) │
└─────────────────────────────────────────────────┘└────────────────────────┘
 help  probe  breach  move  loot  cloak  end  undo
```

### 3.4 Monde et structure du réseau

Une **carte macro** de 14 sites, chacun contenant un graphe de run de 4 à 8 nœuds, ouvert par des événements d'histoire (drapeaux), plus le hub (Planque, Marché). `net` la liste (inconnu / connu / percé / grillé) ; elle sert de panneau latéral en TUI. Les 7 systèmes actuels deviennent 7 des 14 sites (noms conservés, continuité des textes) ; 7 sont à créer.

| Chapitre (titres existants) | Sites (existant, ou **nouveau**) | Quêtes principales (emplacements de `QuestType`, numérotés 1 à 10) |
|---|---|---|
| 1 L'éveil du hacker | Deck (localhost) ; MegaCorp Industries (corp-server-01) | 1 Premiers Pas ; 2 Baptême du Feu |
| 2 Dans l'ombre des corporations | Marché souterrain (underground-market) ; TechDyne Research (research-lab) | 3 Réseaux d'Information |
| 3 Le Projet Aurora | Nexus mainframe (nexus-mainframe) ; **Coffre du Shadow Broker** ; **Clinique NeuroLink** ; MegaCorp Financial (banking-network) ; **Nexus périphérie** | 4 L'œil du Cyclone ; 5 UNDERGROUND_CONTACT ; 6 CORPORATE_SABOTAGE |
| 4 La libération | **Relais d'AURA** ; Gouvernement (gov-database) ; **Station du secteur 7** (infiltration physique) ; **Tour Nexus, cœur d'Aurora** ; **Antenne de diffusion** | 7 AI_LIBERATION ; 8 SHADOW_BROKER ; 9 FINAL_SHOWDOWN ; 10 EPILOGUE (sans run) |

Les dix emplacements de `QuestType` deviennent dix **quêtes principales** (≥ 10 quêtes exigées), chacune décomposée en 1 à 3 scènes ou runs : ≈ 15-18 runs principaux, plus 6 à 8 **contrats annexes** rédigés (un par contact surtout), jamais répétables. Les prérequis suivent l'ordre des emplacements (le test actuel exige « prérequis avant la quête »). Noms de sites : propositions de squelette, modifiables (l'histoire peut évoluer, P4).

### 3.5 Progression

**Paliers** 1 à 6 (noms Novice → Légende conservés), liés aux **quêtes principales terminées** et non à de l'XP : palier 2 après la quête 2, 3 après la 3, 4 après la 4, 5 après la 6, 6 après la 8. Chaque palier donne des cycles (3 → 5), des emplacements de deck, et ouvre des familles de programmes. Pas d'XP par action : le « pas d'impasse d'expérience » (`test_no_experience_dead_end`) devient « chaque palier est atteignable avec la panoplie du palier précédent », vérifié par le solveur. Spécialisations (Spectre, Briseur, Social, Technicien : 3 perks chacune, 1 point par quête principale) : option post-1.0, pas dans le noyau. Réputation par faction (Résistance, Underground, publique) comme monnaie d'histoire, jamais achetable.

### 3.6 Économie, outils et inventaire

Une monnaie (¢), des consommables, des **améliorations de programmes** (niveaux 1-3, ressuscite l'`upgrade_level` mort). Sources : récompense de mission (fixe), butin des nœuds (une fois, drapeau `looted`), contrats annexes. Puits : programmes et améliorations, consommables (Ghost, Spoof, Overclock), services (Notoriété −1 cran, Intel du Shadow Broker, pots-de-vin). Règle chiffrée à faire respecter par le simulateur `[H]` : catalogue permanent total ≥ 1,2 à 1,5 × revenus totaux de la campagne (ordre de grandeur 12-14 k ¢ de revenus, ≈ 15-18 k ¢ de catalogue) : on ne peut pas tout acheter, donc on choisit une panoplie. Correspondance du catalogue actuel :

| Ancien objet (prix) | Devient | Note |
|---|---|---|
| Stealth Module (150) | Module furtif : +1 charge de Cloak | plus de commande cachée |
| Ghost Protocol (80) | consommable Ghost (annule le bruit d'un tour) | plus de menu `laylow` |
| Malware Arsenal (120) | programme Virus, 3 variantes choisies | |
| Proxy Chain Pro (100) | Spoof ×3 charges | |
| Quantum Encryption Key (200) | programme Décryptage | |
| Neural Assistant (500) | slot Compagnon (ECHO), puis AURA par l'histoire | |
| Quantum Chip (800) | programme Quantique | |
| Street Cred Booster (75) | **supprimé** (la réputation ne s'achète pas) ; remplacé par pot-de-vin : Notoriété −1 cran | |
| Neural Accelerator (90) | consommable Overclock | |
| Dark Web VPN (60) | équipement passif : Trace ambiante 2 → 1 par tour | |

**Outils et inventaire** : le joueur possède un **catalogue de programmes** (appris ou achetés, améliorables aux niveaux 1 à 3 : −1 de bruit au niveau 2, +1 de puissance au niveau 3 `[H]`), un **deck** de 4 emplacements au palier 1 à 8 au palier 6, des **équipements passifs** (2 emplacements : VPN, Module furtif…), des **consommables** (5 au plus par type) et 1 emplacement de **Compagnon**. Le deck se compose au **briefing** de chaque mission (`deck`, `equip <programme>`), en s'aidant des familles d'ICE déjà connues (un `probe` précédent, l'Intel du Shadow Broker). `status` et le panneau latéral de la TUI listent exactement ces blocs. Comme la boutique actuelle, la liste dit toujours pourquoi un objet n'est pas disponible (palier, crédits, épuisé) : l'écran n'annonce jamais ce que l'achat refuserait.

### 3.7 Chaleur : Trace (run) et Notoriété (campagne)

- **Trace** : locale à un run (3.3), seuils 30 / 50 / 70 / 100 conservés.
- **Notoriété** 0-100, persistante, par défaut globale (par faction possible plus tard) : bandes **Discret** (0-29), **Surveillé** (30-49), **Traqué** (50-79), **Chassé** (80+). Elle monte de (Trace finale / 4) à chaque run et sur incidents ; elle baisse de 10 par mission terminée et de 25 avec un service payant. Effets : Traqué = +1 de force sur les ICE des sites corporatifs ; Chassé = marché fermé (comme aujourd'hui à 80) et un « raid » narratif (perte de consommables et d'une planque). **Aucun « Attendre » gratuit et illimité**, aucun game over (sauf Hardcore).

### 3.8 Difficulté et accessibilité

Quatre préréglages, chaque paramètre réglable séparément et modifiable en cours de campagne (sauf Hardcore, qui ne se règle plus) :

| Paramètre | Histoire | Normal | Expert | Hardcore |
|---|---|---|---|---|
| Gain de Trace | ×0,5 | ×1 | ×1,25 | ×1,5 |
| Cycles bonus | +1 | 0 | 0 | 0 |
| Prévision (tours à l'avance) | 2 | 1 | 1 (après `probe`) | 1 (après `probe`) |
| `undo` par run | illimité | 3 | 0 | 0 |
| Grillé | rejouer, sans pénalité | rejouer, Notoriété +1 bande, consommables du run perdus | mission rejouée, Notoriété +2 bandes | **game over**, retour à la dernière sauvegarde de hub |
| Indices (`hint`, ECHO-7) | illimités | 3 par mission | 1 par mission | 0 |
| Résolution automatique d'un run (`skip`) | oui | non | non | non |

Accessibilité, contrat de conception :
- **Frontend plain de première classe** : toute information est un texte stable, une ligne par fait, jamais de curseur déplacé ; `NO_COLOR` respecté ; chaque couleur est doublée d'un libellé (« Trace 15/100 (calme) ») ; mode `--ascii` (jauges et cadres sans caractères de dessin) ; **aucun émoji**.
- **Aucune attente dans le moteur.** Vitesse du texte : instantané / rapide / normal / lent, **interruptible** ; défaut : instantané hors terminal ou en plain, rapide en TUI ; la machine à écrire ne touche jamais à l'état.
- **Saisie** : tout choix s'exprime par numéro **ou** par nom, avec complétion (TAB) ; **plus aucune chaîne à recopier ni mot secret** : « décrypter » se fait en choisissant un document dans une liste (`decrypt 3`). Tout secret a un indice en fiction.
- **Aucun temps réel** : le temps ne passe qu'aux commandes ; aucun chronomètre.
- **TUI** : utilisable au clavier seul ; un « journal d'événements » en bas dit en texte tout ce que les widgets montrent ; thème à fort contraste et palette sûre pour le daltonisme (forme et libellé, pas seulement la couleur) ; dégrade proprement sous 80×24 (le panneau se replie).
- **Mode histoire** : `skip` (résolution automatique d'un run, marquée dans la sauvegarde, sans pénalité d'histoire) pour que personne ne soit bloqué par la tactique.

### 3.9 Contacts et missions

Les 9 contacts reçoivent chacun un **rôle de jeu**, un **arc** (2-3 scènes) et une **confiance gagnée par des choix** de dialogue (les libellés inconnu / neutre / amical / de confiance sont conservés), qui ouvre des paliers de service.

| Contact | Rôle de jeu | Arc | Déblocage |
|---|---|---|---|
| ECHO-7 | mentor, tutoriel, service `hint` | l'identité d'ECHO-7 (voir décision 12) | départ |
| R4Z0R | boutique, deck, améliorations | son passé chez MegaCorp, un contrat annexe | fin de la quête 1 (tutoriel) |
| Phoenix | missions d'élite, apprend Exploit | trahi par Nexus ; allié ou rival selon un choix | fin de la quête 3 (Réseaux d'Information) |
| AURA | slot Compagnon, aide en run | sa libération (quête 7) | fin de la quête 4 (L'œil du Cyclone) |
| Shadow Broker | vend de l'Intel (révèle graphe et ICE d'un site) | marchandage, double jeu (quête 8) | quête 5 |
| Neon Angel | médias : « exposer » plutôt que « voler » | la diffusion publique (quête 9) | quête 8 |
| Ghost Walker | accès interne : démarrer un run à un nœud intérieur | loyauté mise à l'épreuve (quête 8) | quête 7 |
| Data Miner | lit les documents, vend Décryptage, transforme du butin en Intel | obsession d'Aurora (quête 5) | quête 4 |
| Nexus Insider | calendrier de sécurité : prévision +1 tour sur Nexus | peut être démasqué (quête 6) | quête 5 |

Dans la colonne « Déblocage », « quête N » signifie : à l'ouverture de la quête N. Les rôles de service sont rendus par des paliers de confiance, pas par un simple niveau.

Une mission est une **suite de scènes** décrites en données (dialogue à choix, run, document, conséquence) avec un petit langage de **conditions** (drapeau, palier, objet, confiance) et d'**effets** (poser un drapeau, gain, déblocage de site/contact, courrier). Les quêtes « à niveau » du moteur actuel survivent : `refresh(&état)` relit l'état, est idempotent et indifférent à l'ordre des événements. Embranchements : une colonne vertébrale unique et **trois fins** assemblées par l'épilogue à partir de trois drapeaux (sort d'AURA, usage des données Aurora, loyauté de Phoenix ou de l'Insider) ; bilan chiffré (runs, Trace maximale, Notoriété, choix). Documents de butin courts (≈ 70 mots), un canal de lore qui n'existait pas (D8).

### 3.10 Correspondance ancien → nouveau

| Ancien | Nouveau |
|---|---|
| 7 nœuds | 7 des 14 sites |
| niveaux 1-6 et XP par action | paliers 1-6 liés aux quêtes principales, plus d'XP par action |
| `bruteforce`, `backdoor`, `uploadvirus`, `exploit`, `aihack`, `quantumdecrypt`, `socialeng`, `temporalhack`, `advhack` | programmes (3.3) et action `breach` |
| `traceroute`, `analyzedefenses`, `scan` | `probe` (dans un run) et `net` (macro) |
| `decrypt` | choix d'un document |
| `stealth`, `stealthmode`, `aiassist`, `neuralsync` | supprimés ; Cloak, Compagnon, Overclock |
| alerte unique, `laylow` (6 méthodes) | Trace + Notoriété, services de la Planque |
| `firewall_strength`, défenses adaptatives, batteries, mode Dieu, quantum cores | supprimés |
| 4 quêtes écrites | 10 quêtes principales (4 chapitres conservés) et 6-8 contrats annexes |
| 1 emplacement de sauvegarde | 3 emplacements + sauvegarde au début de chaque mission + `undo` |

---

## 4. Invariants et tests à conserver comme tests de propriétés

Générateur de base : `proptest` (v1.11.0 sur crates.io `[E]`) produit des **séquences de commandes** (valides ou non) appliquées à partir de parties aléatoires atteignables ; `insta` (1.49.0 `[E]`) pour les transcriptions du frontend plain.

| # | Invariant | Origine C | Propriété dans le moteur Rust |
|---|---|---|---|
| I1 | **Anti-farm** | `test_economy.c` | le moteur tient un registre de gains `Vec<(RewardId, montant)>` : chaque `RewardId` est unique et appartient à l'ensemble rédigé ; après épuisement des sources, aucune séquence de commandes n'augmente crédits, réputation ni objets au-delà du plafond |
| I2 | **Pas d'impasse de progression** | `test_no_experience_dead_end` | solveur : de tout état atteignable, il existe une suite de commandes qui mène à la quête suivante puis à l'épilogue ; chaque mission est solvable avec la panoplie minimale de son palier ; l'échec (grillé) est récupérable |
| I3 | **Aller-retour de sauvegarde** | `test_save.c` | `parse(écrire(s)) == s` pour tout état atteignable ; `écrire(parse(écrire(s))) == écrire(s)` ; clés inconnues ignorées, clés absentes par défaut, version future → erreur, fichier tronqué (marque de fin) → corrompu |
| I4 | **Chargement transactionnel et robuste** | `nh_save_from_text` | un chargement échoué laisse la partie intacte ; octets arbitraires, valeurs hors bornes, noms avec séquences d'échappement : jamais de panique |
| I5 | **Déterminisme** | `--seed` (non tenu en C, D12) | même graine + mêmes commandes ⇒ même hachage d'état et même transcription ; fixtures identiques sous Linux, macOS et Windows (CI) |
| I6 | **Parité i18n** | `test_i18n.c` | mêmes clés FR/EN, mêmes marqueurs de substitution, aucune chaîne vide, aucun émoji (`test_no_emoji_anywhere`), pas de littéral français hors tables ; budget de largeur des libellés qui doivent tenir en TUI |
| I7 | **Table de commandes cohérente** | `test_commands.c` | tout ce qu'affiche l'aide s'exécute ; `Locked` ≠ `Unknown` ; alias uniques ; la complétion ne propose rien de verrouillé (pas de spoiler) |
| I8 | **Récompenses une seule fois, sans récursion** | `test_quests.c`, `test_events.c` | le statut est posé avant le paiement ; `refresh(refresh(s)) == refresh(s)` ; résultat indépendant de l'ordre des événements ; le moteur renvoie une liste d'événements au lieu d'appeler des abonnés (la récursion est impossible par construction) |
| I9 | **Intégrité des tables de contenu** | `test_quests.c`, `test_world.c` | prérequis acycliques, identifiants uniques, tout contact/site/programme référencé existe, toute clé de texte existe en FR et EN ; chaque site est atteignable depuis la racine ; paliers monotones le long des arêtes |
| I10 | **« Une prévision ne ment pas »** | `test_blurbs_match_the_code` | `preview(action)` et `résoudre(action)` appellent la même fonction ; la ligne « Prévision » d'un tour est égale à ce qui arrive |
| I11 | **Bornes et arithmétique** | plafonds `NH_XP_CAP`, crédits 10⁹ | types saturants (crédits, réputation, Trace, Notoriété) ; propriétés avec des montants extrêmes |
| I12 | **Chaleur** | `test_alert.c` | Trace ∈ [0,100] et monotone dans un run ; Notoriété ∈ [0,100] ; seuils strictement ordonnés ; pénalités non décroissantes avec la bande |
| I13 | **Entrées hostiles** | `test_io.c`, `nh_clean_name` | fin d'entrée jamais bouclante ; toute chaîne d'octets donne une commande ou une erreur, sans panique ; un nom assaini n'a ni caractère de contrôle ni plus de 20 caractères |
| I14 | **Perte non sauvegardée / écriture atomique** | `test_save.c` | une partie perdue en Hardcore n'écrase pas la sauvegarde ; un échec d'écriture est signalé une fois |
| I15 | **Tutoriel robuste à l'ordre** | `test_tutorial.c` | les étapes se lisent sur l'état, pas sur la séquence de commandes ; passer le tutoriel ne verse rien |
| I16 | **Rendu sans débordement** | `test_shop_view.c` | pour toute largeur de 80 à 200 et toute hauteur de 24 à 60, aucune ligne ne dépasse la largeur (plain et TUI) ; le rendu ne panique pas |
| I17 | **Nouveau : terminaison d'un run** | — | Trace ambiante > 0 ⇒ tout run se termine en au plus T tours |
| I18 | **Nouveau : `undo` cohérent** | — | `undo` puis la même action redonne le même hachage d'état |
| I19 | **Nouveau : aucun programme dominé** | — | pour chaque programme, il existe une mission (solveur) où le retirer augmente strictement la Trace minimale ; aucun programme n'est toujours meilleur qu'un autre |
| I20 | **Nouveau : économie** | — | catalogue permanent total ≥ 1,2 × revenus totaux maximaux de la campagne ; revenu minimal ≥ coût de la panoplie requise par le palier suivant |
| I21 | **Nouveau : difficulté monotone** | — | marge de Trace du solveur : Histoire ≥ Normal ≥ Expert ; taux de réussite d'un bot : même ordre |
| I22 | **Nouveau : le moteur n'a pas d'horloge** | pauses dispersées en C | aucun accès au temps ni à l'entrée-sortie dans le crate moteur (règle de lint) ; la vitesse du texte ne change aucun hachage d'état |

## 5. Estimation du périmètre du moteur, par système `[H]`

Les fourchettes sont des ordres de grandeur (± 40 %), à raffiner avec la tâche d'architecture. « C actuel » = lignes mesurées (`wc -l`) des modules concernés ; les tests Rust sont attendus du même ordre que le code (le C a 10,8 k lignes de tests unitaires pour 11,4 k lignes de code, 12,3 k avec `strings.def`, hors tests de bout en bout).

| # | Système | C actuel | Rust moteur | Taille | Compl. (1-5) | Risques et dépendances |
|---|---|---|---|---|---|---|
| 1 | Noyau : identifiants, types saturants, PCG32, événements, erreurs | `rng.c` 49, `events` 173 | 600-900 | S | 2 | base de tout ; port direct du PCG32 |
| 2 | **Contenu** : schémas, chargement, validation (missions, sites, programmes, dialogues, références de texte) | tables + `strings.def` 961 | 1 200-1 800 | L | 4 | clé de voûte de « l'histoire évolue » ; format à fixer avec l'architecture |
| 3 | Commandes, analyse, aide, complétion (partagés) | `commands` 343, `complete` 108, `parse` 173 | 700-1 000 | M | 2 | partagé par les deux frontends |
| 4 | **Moteur de run** : graphe, programmes, ICE, Trace, prévision, `undo` | `world` 334, `cmd_hacking` 574, `advanced_hacking` 848, `cmd_advanced` 279 (≈ 60 % mort) | 1 800-2 800 | XL | 5 | équilibrage ; l'espace d'états du solveur croît avec patrouilles et familles |
| 5 | Solveur et vérificateurs de missions (dev et tests) | — | 500-800 | M | 4 | spike OK sur un jouet ; borne à surveiller |
| 6 | Hub : inventaire, deck, boutique, améliorations, consommables | `shop` 205 (+ affichage 517) | 700-1 100 | M | 3 | catalogue à régler par simulation |
| 7 | Progression : paliers, perks optionnels | `progression` 221 | 400-700 | S | 2 | plus simple que l'actuelle |
| 8 | Chaleur (Notoriété) et préréglages de difficulté | `alert` 251 | 350-600 | S | 2 (réglage 4) | |
| 9 | Contacts, dialogues (conditions/effets), confiance, courrier, `hint` | `contacts` 736 | 1 000-1 500 | L | 3 | volume d'écriture |
| 10 | Missions, scènes, drapeaux, épilogue | `quest_system` 592 | 900-1 400 | L | 4 | langage de missions à garder petit |
| 11 | Sauvegarde serde texte versionnée, migrations, chargement transactionnel | `save` 467, `kv` 194 | 500-800 | M | 3 | I3, I4 |
| 12 | Modèle de réglages (difficulté, accessibilité) | `settings` 66 | 250-400 | S | 1 | |
| 13 | Modèle de vue partagé (événements → texte stable) | `term`/`hud`/`shop_view` ≈ 1 100 | 600-1 000 | M | 3 | contrat plain/TUI |
| | **Total moteur** | | **≈ 9 500-14 800** | | | |

Hors moteur : frontend plain 700-1 000, **TUI 2 000-3 000** (journal, carte, deck, Trace, saisie avec complétion), harnais d'équilibrage `neon-sim` 800-1 300 (bots, politiques, métriques de ce rapport).

**Ordre conseillé** (vertical d'abord) : (1) noyau + contenu + commandes + sauvegarde + plain minimal ; (2) missions/contacts/dialogues et **toute la campagne avec runs en `AutoResolve`** (priorité 1) ; (3) moteur de run tactique + solveur + TUI (priorité 2) ; (4) accessibilité intégrée dès (1) : jamais ajoutée après coup (priorité 3) ; (5) simulation et réglage. **Volume de texte** `[H]` (FR, l'anglais en parité) : scènes et briefings ≈ 5,9 k mots, documents de butin (60 × 70) 4,2 k, descriptions de sites et d'ICE 2,1 k, contacts (9 fiches + 4 conversations à choix chacun) 10,4 k, courrier (25 × 120) 3 k, fins et bilan 1,2 k, tutoriel et système 2,5 k : **≈ 29 k mots** (22-33 k) contre ≈ 2,2 k aujourd'hui.

## 6. Décisions à valider

| # | Question | Options | Recommandation |
|---|---|---|---|
| 1 | Direction du modèle de piratage | A infiltration à tours ; B contrats à jets lisibles ; C terminal vivant | **A**, en deux temps (campagne complète en `AutoResolve`, puis tactique) |
| 2 | Place du hasard | déterministe et menaces annoncées ; pourcentages détaillés ; mixte | **déterministe** ; aléa limité au butin et aux variantes |
| 3 | Échec et game over | pas de game over (« grillé » = rejouer avec pénalité) + Hardcore ; game over à 100 ; sauvegarde unique ironman | **pas de game over par défaut** + Hardcore |
| 4 | Progression | paliers liés aux quêtes principales ; XP et niveaux refondus ; XP + spécialisations | **paliers** ; spécialisations après la 1.0 |
| 5 | Envergure | 10 quêtes principales, 15-18 runs, 6-8 contrats, 14 sites, ≈ 4-5 h ; version resserrée (≈ 8 sites, ≈ 2-3 h) ; version étendue (20+ missions) | **ampleur A** avec la version resserrée comme jalon de repli |
| 6 | Contacts | 9 contacts avec rôle, arc et confiance par choix ; contacts décoratifs | **rôles et arcs** |
| 7 | Embranchements et fins | colonne vertébrale + 3 fins par drapeaux ; fin unique ; arbre profond | **3 fins** |
| 8 | Mode histoire (résolution automatique d'un run) | oui, option explicite et sans pénalité ; non ; seulement après plusieurs échecs | **oui**, explicite, marqué dans la sauvegarde |
| 9 | Vitesse du texte par défaut | moteur sans attente, réglage de présentation, défaut instantané en plain et rapide en TUI ; garder les animations d'origine | **réglage de présentation** interruptible |
| 10 | Saisie | numéros + noms + TAB, plus de codes à recopier ni de mots secrets ; garder la saisie libre | **numéros et noms**, plus de secrets |
| 11 | `undo` pendant un run | oui en Histoire et Normal (3 par run en Normal), non en Expert et Hardcore ; jamais ; toujours | **oui, selon la difficulté** |
| 12 | Canon narratif à figer avant d'écrire | ECHO-7 est une IA / ancien prototype Aurora (indice : l'« ECHO-Assistant v3.1 », assistant nommé ECHO, dans `advanced_hacking.c`) ; ECHO-7 est humain ; laisser ouvert. Dans tous les cas : renommer « Agent Smith », réserver « Ghost Protocol » aux outils (le projet reste Aurora), et dire que seule une partie d'AURA est libre au niveau 5 (la quête 7, « libération », libère le cœur) | **ECHO-7 est une IA** (à valider) et les trois renommages |

---

## Annexe A. Méthode et reproductibilité

Tout a été fait dans la copie `<scratchpad>/nh-copy` (jamais dans le dépôt).

- **Compilation et tests** : `make -j8` puis `make -j8 unit` : 26 suites, 0 échec `[E]` (alert 324, commands 1 140, contacts 415, economy 3 194, hud 5 408, i18n 1 319, quests 474, rng 20 019, world 984…). e2e et pty non rejoués.
- **`tools/bot.c`** : joue le vrai jeu via `nh_dispatch` (état du jeu réel, entrées injectées avec `nh_feed`), graines `srand(1..N)` (`rand()` de la glibc : résultats reproductibles sur cette libc seulement). Stratégies `focused` (scan + bruteforce) et `complete` (tout le contenu) ; politiques d'alerte prudente / équilibrée / hardie / aucune ; 500 graines par ligne des tableaux 2.3-2.4.
- **`tools/probe.c`** : valeurs exactes de `nh_world_chance`, `calculate_hack_success_rate`, refroidissement gratuit, récompenses de `backdoor`/`uploadvirus`/`quantumdecrypt`, totaux de revenus.
- **`tools/probe2.c`** et `bot_t.c` : durées d'animation, mesurées en interceptant `nh_sleep_ms` à l'édition de liens (`-Wl,--wrap`).
- **Spike Rust** `…/scratchpad/runspike/` (édition 2024, aucune dépendance) : modèle jouet de run (6 nœuds, programmes Brute/Exploit/Cloak, Sentinelles, Trace ambiante), recherche exhaustive (3 071 546 états, 3,2 s en release) et Dijkstra avec la Trace comme coût (8 393 clés, 3,4 ms) : mêmes optimums (24, 28, 24, 33, 17) sur les cinq jeux de règles testés.
- **Non vérifié** : tout ce qui est marqué `[H]` (tailles, volumes de texte, notes de priorité, valeurs de départ du modèle de run, ordre de grandeur des revenus), la durée de partie visée (4-5 h), et le comportement du solveur sur un modèle complet (patrouilles, familles d'ICE, 8 nœuds).
