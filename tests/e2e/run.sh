#!/usr/bin/env bash
# Tests de bout en bout : lancent le vrai exécutable et vérifient ce qu'il fait
# (codes de retour, sorties, robustesse). Contrairement aux anciens scripts du
# projet, chaque test a une assertion et un échec fait échouer le script.
#
#   NEON_HACK_BIN=./neon_hack tests/e2e/run.sh

set -u

BIN=${NEON_HACK_BIN:-./neon_hack}
LIMIT=${NEON_HACK_TIMEOUT:-20}
MAX_BYTES=200000

if [ ! -x "$BIN" ]; then
    echo "exécutable introuvable : $BIN (lancez 'make' d'abord)" >&2
    exit 2
fi

passed=0
failed=0

pass() { passed=$((passed + 1)); printf '  ok    %s\n' "$1"; }
fail() {
    failed=$((failed + 1))
    printf '  FAIL  %s\n' "$1"
    [ -n "${2:-}" ] && printf '        %s\n' "$2"
}

ESC=$(printf '\033')
strip_ansi() { sed "s/${ESC}\[[0-9;]*m//g"; }

# Chaque exécution reçoit son propre dossier de données (--data-dir) : les tests ne touchent
# jamais aux vraies sauvegardes du joueur et ne se gênent pas entre eux.
DATA=$(mktemp -d)
trap 'rm -rf "$DATA"' EXIT
RUNS=0
LANGOPT=(--lang fr)

# run_in <dossier|-> <entrée> [options...] : lance le jeu TEL QUEL (menu compris) avec ce dossier
# de données (« - » : celui de l'environnement). Met la sortie (stdout+stderr, sans couleurs,
# bornée) dans $OUT et le code de retour du jeu dans $CODE. Le français est la langue par défaut
# des tests (indépendant de $LANG, forcé à C) ; run_free ne passe pas --lang.
#
# La sortie est bornée par `head -c` : si le jeu se remet à boucler à l'infini,
# il reçoit SIGPIPE (code 141) au lieu de remplir le disque.
run_in() {
    local dir=$1 input=$2 tmp datadir=()
    shift 2
    [ "$dir" = "-" ] || datadir=(--data-dir "$dir")
    tmp=$(mktemp)
    printf '%b' "$input" | LANG=${GAME_LANG:-C} timeout "$LIMIT" "$BIN" ${LANGOPT[@]+"${LANGOPT[@]}"} \
        ${datadir[@]+"${datadir[@]}"} "$@" 2>&1 | head -c "$MAX_BYTES" >"$tmp"
    CODE=${PIPESTATUS[1]}
    OUT=$(strip_ansi <"$tmp")
    rm -f "$tmp"
}

# run <entrée> [options...] : une partie neuve dans un dossier jetable (--new, sans menu). La
# première ligne de <entrée> est le nom du héros ; run() y ajoute la confirmation (Entrée) et le
# choix « je me débrouille » (2) du prologue, pour que les tests partent directement de la boucle
# de jeu. Une entrée vide reste vide (fin de fichier dès la question du nom).
run() {
    local input=$1 first rest
    shift
    if [ -n "$input" ]; then
        first=${input%%\\n*}
        if [ "$first" = "$input" ]; then rest=''; else rest=${input#*\\n}; fi
        input="${first}\\n\\n2\\n${rest}"
    fi
    RUNS=$((RUNS + 1))
    run_in "$DATA/run$RUNS" "$input" --new "$@"
}

# run_free <dossier|-> <entrée> [options...] : comme run_in, mais sans --lang : la langue vient alors
# de $LANG (GAME_LANG, « C » par défaut = anglais) et des réglages enregistrés. Les variables
# d'environnement se passent en préfixe : GAME_LANG=fr_FR.UTF-8 HOME=... run_free ...
run_free() {
    local LANGOPT=()
    run_in "$@"
}

contains() { printf '%s' "$OUT" | grep -qF -- "$1"; }

# --- Options de ligne de commande -------------------------------------------

OUT=$("$BIN" --version 2>&1); CODE=$?
if [ "$CODE" -eq 0 ] && printf '%s' "$OUT" | grep -qE '^neon_hack [0-9]'; then
    pass "--version affiche la version et retourne 0"
else fail "--version" "code=$CODE sortie='$OUT'"; fi

OUT=$("$BIN" --help 2>&1); CODE=$?
if [ "$CODE" -eq 0 ] && contains "--seed" && contains "--lang"; then
    pass "--help liste les options et retourne 0"
else fail "--help" "code=$CODE"; fi

OUT=$("$BIN" --bogus 2>&1 >/dev/null); CODE=$?
if [ "$CODE" -eq 2 ] && contains "unknown option: --bogus"; then
    pass "option inconnue : erreur claire et code 2"
else fail "option inconnue" "code=$CODE sortie='$OUT'"; fi

OUT=$("$BIN" --seed abc 2>&1 >/dev/null); CODE=$?
if [ "$CODE" -eq 2 ] && contains "--seed"; then
    pass "--seed invalide : erreur claire et code 2"
else fail "--seed invalide" "code=$CODE sortie='$OUT'"; fi

# --- Fin d'entrée : ne doit jamais boucler (bug d'origine : 2,6 Go en 25 s) -----

run '' --fast
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ] && contains "Entrée fermée"; then
    pass "entrée vide : le jeu s'arrête proprement (${#OUT} octets)"
else fail "entrée vide" "code=$CODE taille=${#OUT} (124 = boucle infinie)"; fi

run 'Testeur\n' --fast --lang fr
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ] && contains "Entrée fermée"; then
    pass "EOF après le prologue : fin de session annoncée"
else fail "EOF après le prologue" "code=$CODE taille=${#OUT}"; fi

run 'Testeur\ncontact ECHO-7\n' --fast
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ]; then
    pass "EOF au milieu d'un dialogue : pas de boucle"
else fail "EOF dans un dialogue" "code=$CODE taille=${#OUT}"; fi

# --- Session de base ---------------------------------------------------------

run 'Testeur\nstatus\nquit\n' --fast --lang fr
if [ "$CODE" -eq 0 ] && contains "Niveau: 1" && contains "Niveau d'alerte" && contains "Merci d'avoir joué"; then
    pass "session minimale : status puis quit"
else fail "session minimale" "code=$CODE"; fi

run '\nstatus\nquit\n' --fast
if contains "Nom: Case"; then
    pass "nom vide : le héros s'appelle « Case » par défaut"
else fail "nom vide" "pas de nom par défaut"; fi

run 'Testeur\nquit\n' --fast --lang en
if contains "Thanks for playing"; then pass "--lang en : messages de fin en anglais"
else fail "--lang en"; fi

run 'Testeur\nquit\n' --fast --lang fr
if contains "Merci d'avoir joué"; then pass "--lang fr : messages de fin en français"
else fail "--lang fr"; fi

SCAN5='T\nscan\nscan\nscan\nscan\nscan\nquit\n'
run "$SCAN5" --fast
if [ "$CODE" -eq 0 ] && contains "NIVEAU SUPÉRIEUR"; then
    pass "5 scans : montée au niveau 2"
else fail "progression" "code=$CODE"; fi

# --- Robustesse des saisies --------------------------------------------------

run 'T\ncontact ECHO-7\nabc\nquit\n' --fast
if [ "$CODE" -eq 0 ] && contains "Option invalide" && ! contains "runtime error"; then
    pass "dialogue : saisie non numérique rejetée proprement"
else fail "saisie non numérique" "code=$CODE"; fi

LONG=$(head -c 500 /dev/zero | tr '\0' a)
run "T\n${LONG}\nstatus\nquit\n" --fast
unknown=$(printf '%s' "$OUT" | grep -c "Commande inconnue")
if [ "$CODE" -eq 0 ] && [ "$unknown" -eq 1 ] && contains "Niveau: 1"; then
    pass "ligne de 500 caractères : tronquée sans polluer la commande suivante"
else fail "ligne trop longue" "code=$CODE commandes_inconnues=$unknown"; fi

# --- Couche de commandes (table unique) -------------------------------------

run 'T\nSCAN\n  scan  \nquit\n' --fast
if [ "$CODE" -eq 0 ] && ! contains "Commande inconnue"; then
    pass "commandes insensibles à la casse et aux espaces"
else fail "casse/espaces" "code=$CODE"; fi

run 'T\nfoobar\nquit\n' --fast
if contains "Commande inconnue: foobar" && contains "help"; then
    pass "commande inconnue : message et renvoi vers help"
else fail "commande inconnue"; fi

run 'T\nbruteforce localhost\nquit\n' --fast
if contains "Commande non disponible à votre niveau." && ! contains "Commande inconnue"; then
    pass "commande verrouillée : message distinct d'une commande inconnue"
else fail "commande verrouillée"; fi

run 'T\nadvhack x\nsocialeng x\nquit\n' --fast
if contains "niveau 3 requis" && contains "niveau 2 requis"; then
    pass "commande à niveau minimum : le niveau requis est annoncé"
else fail "niveau requis"; fi

run 'T\nadvhack x\nquit\n' --fast --lang en
if contains "level 3 required"; then pass "niveau requis annoncé en anglais"
else fail "niveau requis (en)"; fi

run 'T\nhelp\nquit\n' --fast
if contains "COMMANDES DISPONIBLES" && contains "scan" && ! contains "bruteforce" && ! contains "advhack"; then
    pass "help : seulement les commandes débloquées"
else fail "help au niveau 1"; fi

run 'T\nscan\nscan\nscan\nscan\nscan\nhelp\nquit\n' --fast
if contains "bruteforce"; then pass "help : bruteforce apparaît une fois débloquée"
else fail "help après déblocage"; fi

run 'T\nhelp\nquit\n' --fast --lang en
if contains "AVAILABLE COMMANDS" && contains "Scan the network"; then pass "help en anglais"
else fail "help (en)"; fi

# Les écrans d'origine (logo, intro) ont encore des couleurs codées en dur : on ne
# vérifie ici que la partie déjà migrée (de l'aide jusqu'à la fin du statut).
raw=$(printf 'T\n\n2\nhelp\nstatus\nquit\n' | "$BIN" --fast --lang fr --no-color --new --data-dir "$DATA/nocolor" 2>&1 \
    | awk '/COMMANDES DISPONIBLES/{on=1} on{print} /Niveau d.alerte/{on=0}' | grep -c "$ESC")
if [ "$raw" -eq 0 ]; then pass "--no-color : aucune séquence d'échappement dans help/status"
else fail "--no-color help/status" "$raw lignes avec ESC"; fi

run 'T\nstatus\nquit\n' --fast --lang en
if contains "Level: 1" || contains "Level:"; then pass "status en anglais"
else fail "status (en)"; fi

run 'T\nexit\n' --fast
if [ "$CODE" -eq 0 ] && contains "Merci d'avoir joué"; then pass "exit : alias de quit"
else fail "alias exit"; fi

run 'T\nEXIT\n' --fast
if [ "$CODE" -eq 0 ] && contains "Merci d'avoir joué"; then pass "EXIT en majuscules"
else fail "EXIT"; fi

run 'T\ndecrypt WKLV#LV#D#WHVW\nquit\n' --fast
if contains "Commande non disponible"; then pass "decrypt verrouillée au départ"
else fail "decrypt verrouillée"; fi

# --- Progression ------------------------------------------------------------------

# Avant : 30 scans menaient au niveau 8 (aucun plafond, scan sans limite d'expérience).
SCANS30='T\n'$(printf 'scan\\n%.0s' $(seq 1 30))'status\nquit\n'
run "$SCANS30" --fast
if contains "Niveau: 2 (Apprenti)" && contains "Expérience: 15/60" && contains "déjà cartographié"; then
    pass "30 scans : niveau 2 et 15 XP, pas d'expérience infinie"
else fail "scan sans limite" "$(printf '%s' "$OUT" | grep -E 'Niveau:|Expérience:' | tr '\n' ' ')"; fi

run 'T\nscan\nscan\nscan\nscan\nscan\nstatus\nquit\n' --fast --lang en
if contains "LEVEL UP" && contains "level 2: Apprentice" && contains "New commands unlocked" && contains "bruteforce"; then
    pass "montée de niveau annoncée en anglais avec les commandes débloquées"
else fail "montée de niveau (en)"; fi

# --- Alerte (échelle unique 0-100) ---------------------------------------------

# Bug d'origine : trois scans affichaient « alerte 0/100 » car la valeur était réécrasée.
# Le scan ajoute 1 ; le refroidissement (1 par action de hacking) le retire à l'action
# suivante : le niveau reste bas mais l'annonce et la jauge existent.
run 'T\nscan\nstatus\nquit\n' --fast
if contains "[ALERTE +1]" && contains "Niveau d'alerte:" && contains "1/100"; then
    pass "un scan annonce +1 et le statut affiche 1/100 (l'alerte n'est plus écrasée)"
else fail "alerte après un scan"; fi

run 'T\nscan\nstatus\nquit\n' --fast --lang en
if contains "[ALERT +1]" && contains "Alert level:" && contains "1/100"; then
    pass "alerte en anglais"
else fail "alerte (en)"; fi

run 'T\nlaylow\n1\nlaylow\n0\nlaylow\nabc\nlaylow\n9\nquit\n' --fast
if [ "$CODE" -eq 0 ] && contains "RÉDUCTION D'ALERTE" && contains "Vous restez dans l'ombre" \
    && [ "$(printf '%s' "$OUT" | grep -c 'Choix invalide')" -eq 2 ]; then
    pass "laylow : menu, annulation et saisies invalides"
else fail "laylow" "code=$CODE"; fi

run 'T\nlaylow\n' --fast
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ]; then
    pass "laylow : fin d'entrée dans le menu sans boucle"
else fail "laylow EOF" "code=$CODE"; fi

# --- Reproductibilité et rapidité -------------------------------------------

BRUTE='T\nscan\nscan\nscan\nscan\nscan\nbruteforce localhost\nbruteforce localhost\nquit\n'

run "$BRUTE" --fast --seed 42; A=$OUT
run "$BRUTE" --fast --seed 42; B=$OUT
if [ -n "$A" ] && [ "$A" = "$B" ]; then
    pass "--seed 42 deux fois : sorties identiques"
else fail "reproductibilité" "deux parties avec la même graine diffèrent"; fi

distinct=0
run "$BRUTE" --fast --seed 1; first=$OUT
for seed in 2 3 4 5 6 7 8; do
    run "$BRUTE" --fast --seed "$seed"
    [ "$OUT" != "$first" ] && distinct=1
done
if [ "$distinct" -eq 1 ]; then
    pass "graines différentes : parties différentes (la graine a un effet)"
else fail "effet de la graine" "8 graines, une seule issue"; fi

start=$SECONDS
run "$BRUTE" --seed 42
elapsed=$((SECONDS - start))
if [ "$CODE" -eq 0 ] && [ "$elapsed" -le 3 ]; then
    pass "sortie redirigée = mode rapide automatique (${elapsed}s)"
else fail "mode rapide" "code=$CODE durée=${elapsed}s (attendu <= 3s ; l'ancienne version dépassait 15s)"; fi

# --- Monde unifié : relais, découverte, exploit ------------------------------

FIVE='scan\nscan\nscan\nscan\nscan\n'

run "T\n${FIVE}scan\nquit\n" --fast
if contains "corp-server-01" && contains "[NOUVEAU]" && contains "[ROUTE FERMÉE]" && ! contains "nexus-mainframe"; then
    pass "scan au niveau 2 : nouveau système visible, route fermée, niveau 3 encore caché"
else fail "scan par niveau"; fi

run "T\n${FIVE}scan\nbruteforce corp-server-01\nquit\n" --fast
if contains "Route fermée vers corp-server-01 : compromettez d'abord localhost." && ! contains "ATTAQUE BRUTE FORCE"; then
    pass "bruteforce sur un système dont le relais n'est pas compromis : refusé, avec explication"
else fail "route fermée"; fi

run "T\n${FIVE}bruteforce corp-server-01\nquit\n" --fast
if contains "Système 'corp-server-01' introuvable"; then
    pass "monter de niveau ne révèle rien tout seul : il faut relancer un scan"
else fail "découverte par scan"; fi

run "T\n${FIVE}scan\nbruteforce nexus-mainframe\nquit\n" --fast
if contains "Système 'nexus-mainframe' introuvable" && ! contains "ATTAQUE BRUTE FORCE"; then
    pass "système pas encore découvert (niveau trop bas) : introuvable"
else fail "système non découvert"; fi

run "T\n${FIVE}scan\nbruteforce corp-server-01\nquit\n" --fast --lang en
if contains "No route to corp-server-01: compromise localhost first."; then
    pass "route fermée expliquée en anglais"
else fail "route fermée (en)"; fi

run "T\nexploit localhost\nquit\n" --fast
if contains "Commande non disponible à votre niveau."; then pass "exploit verrouillée avant le niveau 4"
else fail "exploit verrouillée"; fi

# Avec une graine où localhost tombe : la route vers corp-server-01 s'ouvre, l'état est persistant.
ok_seed=""
for seed in 1 2 3 4 5 6 7 8; do
    run "T\n${FIVE}bruteforce localhost\nbruteforce corp-server-01\nscan\nquit\n" --fast --seed "$seed"
    if contains "Accès obtenu à localhost !"; then ok_seed=$seed; break; fi
done
if [ -n "$ok_seed" ] && ! contains "Route fermée" && contains "[COMPROMIS]" && contains "Données récupérées : 10 crédits"; then
    pass "compromettre le relais ouvre la route ; le scan garde l'état (graine $ok_seed)"
else fail "ouverture de route" "graine=${ok_seed:-aucune}"; fi

# Un système compromis ne se re-pirate pas, et ne repaie pas.
if [ -n "$ok_seed" ]; then
    run "T\n${FIVE}bruteforce localhost\nbruteforce localhost\nbruteforce localhost\nquit\n" --fast --seed "$ok_seed"
    paid=$(printf '%s' "$OUT" | grep -c "Données récupérées")
    again=$(printf '%s' "$OUT" | grep -c "Système déjà compromis")
    if [ "$paid" -eq 1 ] && [ "$again" -eq 2 ]; then
        pass "re-pirater un système compromis : ni gain ni nouvelle expérience (1 paiement, 2 refus)"
    else fail "pas de re-paiement" "paiements=$paid refus=$again"; fi
fi

# --- Menu de lancement, sauvegarde, réglages ---------------------------------------------------

file_has() { grep -qF -- "$2" "$1" 2>/dev/null; }

# Menu : entrée piped, un seul choix « Quitter ».
D="$DATA/menu"
run_in "$D" '0\n'
if [ "$CODE" -eq 0 ] && contains "MENU PRINCIPAL" && contains "Continuer (aucune sauvegarde)" \
    && contains "Nouvelle partie" && contains "Langue : Français" && contains "Options" \
    && contains "Merci d'avoir joué" && ! contains "Entrée fermée"; then
    pass "menu : affiché au lancement, « Quitter » ferme proprement"
else fail "menu au lancement" "code=$CODE"; fi

run_in "$DATA/menu-eof" ''
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ] && contains "Entrée fermée"; then
    pass "menu : fin d'entrée = sortie propre, sans boucle"
else fail "menu EOF" "code=$CODE taille=${#OUT}"; fi

run_in "$DATA/menu-bad" 'abc\n9\n-1\n0\n'
if [ "$CODE" -eq 0 ] && [ "$(printf '%s' "$OUT" | grep -c 'Option invalide')" -eq 3 ]; then
    pass "menu : choix invalides redemandés (3 rejets), puis Quitter"
else fail "menu choix invalides" "code=$CODE"; fi

run_in "$DATA/menu-nosave" '1\n0\n'
if [ "$CODE" -eq 0 ] && contains "Aucune sauvegarde à charger" && [ "$(printf '%s' "$OUT" | grep -c 'MENU PRINCIPAL')" -eq 2 ]; then
    pass "menu : « Continuer » sans sauvegarde explique et reste au menu"
else fail "continuer sans sauvegarde" "code=$CODE"; fi

# Nouvelle partie par le menu : nom choisi dans le prologue, sauvegarde créée.
D="$DATA/game1"
run_in "$D" '2\nMolly\n\n2\nstatus\nquit\n'
if [ "$CODE" -eq 0 ] && contains "ECHO-7" && contains "« Molly », donc" && contains "Nom: Molly" \
    && contains "Partie sauvegardée." && [ -f "$D/savegame.sav" ] \
    && file_has "$D/savegame.sav" "player.name=Molly" && file_has "$D/savegame.sav" "end=1"; then
    pass "nouvelle partie : le nom se choisit dans le prologue, la sauvegarde est écrite"
else fail "nouvelle partie via le menu" "code=$CODE"; fi

if [ ! -e "$D/savegame.sav.tmp" ]; then pass "sauvegarde atomique : aucun fichier temporaire ne reste"
else fail "fichier temporaire oublié"; fi

# Continuer dans une nouvelle session : même héros, même progression.
D="$DATA/game2"
run_in "$D" '2\nMolly\n\n2\nscan\nscan\nstatus\nquit\n'
before=$(printf '%s' "$OUT" | grep -E 'Expérience:|Crédits:|Nom:' | tail -3)
run_in "$D" '1\nstatus\nquit\n'
after=$(printf '%s' "$OUT" | grep -E 'Expérience:|Crédits:|Nom:' | tail -3)
if [ "$CODE" -eq 0 ] && [ -n "$before" ] && [ "$before" = "$after" ] && contains "Molly · niveau 1"; then
    pass "continuer : le héros et sa progression sont retrouvés d'une session à l'autre"
else fail "continuer" "avant='$before' après='$after'"; fi

# Entrée seule = Continuer quand une sauvegarde existe.
run_in "$D" '\nstatus\nquit\n'
if [ "$CODE" -eq 0 ] && contains "Nom: Molly" && ! contains "ECHO-7 » Enfin"; then
    pass "menu : Entrée seule reprend la partie sauvegardée"
else fail "Entrée = continuer" "code=$CODE"; fi

# La partie est enregistrée après chaque commande : pas besoin de quitter proprement.
D="$DATA/game3"
run_in "$D" '2\nNeo\n\n2\nscan\nscan\nscan\n'
if [ "$CODE" -eq 0 ] && file_has "$D/savegame.sav" "player.scans_done=3"; then
    pass "sauvegarde automatique : la partie survit à une fin brutale (3 scans enregistrés)"
else fail "sauvegarde automatique" "code=$CODE"; fi
run_in "$D" '1\nstatus\nquit\n'
if contains "Nom: Neo" && contains "Expérience: 12/15"; then
    pass "reprise après fin brutale : progression intacte"
else fail "reprise après fin brutale" "$(printf '%s' "$OUT" | grep -E 'Expérience:' | tr '\n' ' ')"; fi

# Écraser une sauvegarde demande confirmation.
D="$DATA/game4"
run_in "$D" '2\nMolly\n\n2\nquit\n'
cp "$D/savegame.sav" "$DATA/game4.bak"
run_in "$D" '2\nn\n0\n'
if [ "$CODE" -eq 0 ] && contains "remplacera" && cmp -s "$D/savegame.sav" "$DATA/game4.bak"; then
    pass "nouvelle partie sur une sauvegarde : confirmation, refus = rien ne change"
else fail "confirmation d'écrasement (refus)" "code=$CODE"; fi
run_in "$D" '2\noui\nJack\n\n2\nstatus\nquit\n'
if contains "Nom: Jack" && file_has "$D/savegame.sav" "player.name=Jack"; then
    pass "nouvelle partie : après confirmation, l'ancienne sauvegarde est remplacée"
else fail "confirmation d'écrasement (accord)"; fi

# Sauvegarde abîmée : le menu le dit, ne plante pas, ne détruit rien.
D="$DATA/bad1"; mkdir -p "$D"
printf 'ceci n est pas une sauvegarde\n' >"$D/savegame.sav"
run_in "$D" '1\n0\n'
if [ "$CODE" -eq 0 ] && contains "sauvegarde illisible" && contains "corrompue" \
    && file_has "$D/savegame.sav" "ceci n est pas"; then
    pass "sauvegarde corrompue : signalée au menu, fichier conservé"
else fail "sauvegarde corrompue" "code=$CODE"; fi

D="$DATA/bad2"
run_in "$D" '2\nMolly\n\n2\nquit\n'
size=$(wc -c <"$D/savegame.sav")
head -c $((size / 2)) "$D/savegame.sav" >"$D/half"; mv "$D/half" "$D/savegame.sav"
run_in "$D" '1\n0\n'
if [ "$CODE" -eq 0 ] && contains "corrompue"; then
    pass "sauvegarde tronquée : refusée, jamais chargée à moitié"
else fail "sauvegarde tronquée" "code=$CODE"; fi

D="$DATA/bad3"; mkdir -p "$D"
printf 'neon-hack-save=99\nend=1\n' >"$D/savegame.sav"
run_in "$D" '1\n0\n'
if [ "$CODE" -eq 0 ] && contains "version plus récente"; then
    pass "sauvegarde d'une version future : message dédié"
else fail "sauvegarde trop récente" "code=$CODE"; fi

D="$DATA/bad4"
run_in "$D" '2\nMolly\n\n2\nquit\n'
sed -i.orig 's/^player.level=.*/player.level=99/' "$D/savegame.sav" && rm -f "$D/savegame.sav.orig"
run_in "$D" '1\n0\n'
if [ "$CODE" -eq 0 ] && contains "corrompue"; then
    pass "valeur hors limites (niveau 99) : sauvegarde refusée"
else fail "valeur hors limites" "code=$CODE"; fi

# --new : nouvelle partie tout de suite, sans menu.
run_in "$DATA/new1" 'Trinity\n\n2\nstatus\nquit\n' --new
if [ "$CODE" -eq 0 ] && ! contains "MENU PRINCIPAL" && contains "Nom: Trinity"; then
    pass "--new : prologue directement, sans menu"
else fail "--new" "code=$CODE"; fi

# Prologue : refus de confirmation, séquences d'échappement, nom vide.
run_in "$DATA/name1" 'Molly\nn\nJack\n\n2\nstatus\nquit\n' --new
if contains "reprenons" && contains "Nom: Jack" && ! contains "Nom: Molly"; then
    pass "prologue : refuser la confirmation redemande le nom"
else fail "nom : nouvelle tentative"; fi

raw=$(printf '\033[2JPirate\n\n2\nstatus\nquit\n' | "$BIN" --lang fr --new --data-dir "$DATA/name2" 2>&1 | grep -c "${ESC}\[2J")
if [ "$raw" -eq 0 ]; then pass "prologue : une séquence d'échappement saisie comme nom n'atteint pas l'écran"
else fail "nom avec ESC" "$raw occurrences"; fi

run_in "$DATA/name3" '     \n\n2\nstatus\nquit\n' --new
if contains "Nom: Case"; then pass "prologue : un nom d'espaces vaut « Case »"
else fail "nom d'espaces"; fi

run_in "$DATA/name4" 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\n\n2\nstatus\nquit\n' --new
if contains "Nom: ABCDEFGHIJKLMNOPQRST" && ! contains "Nom: ABCDEFGHIJKLMNOPQRSTU"; then
    pass "prologue : nom limité à 20 caractères"
else fail "nom trop long"; fi

run_in "$DATA/name5" 'Zoë\n\n2\nstatus\nquit\n' --new
if contains "Nom: Zoë"; then pass "prologue : nom accentué conservé"
else fail "nom accentué"; fi

eof_ok=1
i=0
for input in 'Molly' 'Molly\n' 'Molly\nn\n' 'Molly\n\n' 'Molly\n\nzzz\n'; do
    i=$((i + 1))
    run_in "$DATA/eof$i" "$input" --new
    if [ "$CODE" -ne 0 ] || [ "${#OUT}" -ge 10000 ] || ! contains "Entrée fermée"; then
        eof_ok=0
        fail "EOF dans le prologue ($input)" "code=$CODE taille=${#OUT}"
    fi
done
[ "$eof_ok" -eq 1 ] && pass "prologue : fin d'entrée à chaque question = sortie propre"

# Langue et options : persistantes d'une session à l'autre.
D="$DATA/settings1"
GAME_LANG=C run_free "$D" '3\n0\n'
if [ "$CODE" -eq 0 ] && contains "MAIN MENU" && contains "MENU PRINCIPAL" && file_has "$D/settings.cfg" "lang=fr"; then
    pass "menu : la langue se change à chaud (EN → FR) et s'enregistre"
else fail "choix de la langue" "code=$CODE"; fi

GAME_LANG=C run_free "$D" '0\n'
if contains "MENU PRINCIPAL" && ! contains "MAIN MENU"; then
    pass "langue enregistrée : retrouvée au lancement suivant (malgré LANG=C)"
else fail "langue persistante"; fi

GAME_LANG=C run_free "$D" '0\n' --lang en
if contains "MAIN MENU" && ! contains "MENU PRINCIPAL" && file_has "$D/settings.cfg" "lang=fr"; then
    pass "--lang l'emporte sur le réglage enregistré, sans l'effacer"
else fail "--lang contre réglage"; fi

D="$DATA/settings2"
run_in "$D" '4\n1\n2\n0\n0\n'
if [ "$CODE" -eq 0 ] && contains "OPTIONS" && file_has "$D/settings.cfg" "color=0" && file_has "$D/settings.cfg" "fast=1"; then
    pass "options : couleurs et animations se règlent et s'enregistrent"
else fail "options" "code=$CODE"; fi

D="$DATA/settings3"; mkdir -p "$D"
printf 'lang=klingon\ncolor=peut-être\nfast=1\nn importe quoi\n' >"$D/settings.cfg"
GAME_LANG=C run_free "$D" '0\n'
if [ "$CODE" -eq 0 ] && contains "MAIN MENU"; then
    pass "réglages abîmés : valeurs invalides ignorées, le jeu démarre"
else fail "réglages abîmés" "code=$CODE"; fi

# Où vivent les fichiers.
X="$DATA/xdg"
XDG_DATA_HOME="$X" HOME="$DATA/home" run_free - '2\nMolly\n\n2\nquit\n'
if [ "$CODE" -eq 0 ] && [ -f "$X/neon-hack/savegame.sav" ] && [ ! -e "$DATA/home" ]; then
    pass "sans --data-dir : XDG_DATA_HOME/neon-hack (le dossier est créé)"
else fail "dossier XDG" "code=$CODE"; fi

XDG_DATA_HOME="" HOME="$DATA/home2" run_free - '2\nMolly\n\n2\nquit\n'
if [ "$CODE" -eq 0 ] && [ -f "$DATA/home2/.local/share/neon-hack/savegame.sav" ]; then
    pass "sans XDG_DATA_HOME : \$HOME/.local/share/neon-hack"
else fail "dossier HOME" "code=$CODE"; fi

XDG_DATA_HOME="" HOME="" run_in - '1\n2\nMolly\n\n2\nstatus\nquit\n'
if [ "$CODE" -eq 0 ] && contains "Aucun dossier de données" && contains "Nom: Molly" && ! contains "Impossible de sauvegarder"; then
    pass "aucun dossier de données : le jeu se joue quand même, sans sauvegarde"
else fail "sans dossier de données" "code=$CODE"; fi

# Dossier impossible à écrire : signalé, la partie continue.
printf 'un fichier\n' >"$DATA/fichier"
run_in "$DATA/fichier/sous" 'Molly\n\n2\nstatus\nquit\n' --new
if [ "$CODE" -eq 0 ] && contains "Impossible de sauvegarder" && contains "Nom: Molly" && contains "Merci d'avoir joué"; then
    pass "sauvegarde impossible : avertissement, la partie continue"
else fail "dossier non inscriptible" "code=$CODE"; fi

# La sauvegarde ne contient rien d'inattendu (texte pur, pas de séquence d'échappement).
D="$DATA/game2"
if [ "$(grep -c "$ESC" "$D/savegame.sav")" -eq 0 ] && head -1 "$D/savegame.sav" | grep -q '^neon-hack-save=1$'; then
    pass "fichier de sauvegarde : texte lisible, version en première ligne"
else fail "format de sauvegarde"; fi

# --- Tutoriel (ECHO-7) --------------------------------------------------------------------

TUTO_HEAD='Neo\n\n1\n'

# Le prologue propose le tutoriel ; ECHO-7 donne la première consigne.
run_in "$DATA/tuto0" "${TUTO_HEAD}quests\nquit\n" --new
if [ "$CODE" -eq 0 ] && contains "Montre-moi les ficelles" && contains "Je me débrouille" \
    && contains "Tape 'quests'" && contains "=== MISSION EN COURS ===" && contains "[>] Ouvrir le journal de mission" \
    && contains "Objectif accompli : Ouvrir le journal de mission"; then
    pass "tutoriel : proposé au prologue, première consigne d'ECHO-7, journal de mission"
else fail "début du tutoriel" "code=$CODE"; fi

run_in "$DATA/tuto0b" 'Neo\n\n\nquests\nquit\n' --new
if contains "=== MISSION EN COURS ==="; then pass "tutoriel : Entrée au choix = tutoriel (par défaut)"
else fail "tutoriel par défaut"; fi

# « Je me débrouille » : pas de tutoriel, le journal de quêtes d'origine.
run_in "$DATA/tuto1" 'Neo\n\n2\nquests\n0\nquit\n' --new
if [ "$CODE" -eq 0 ] && ! contains "MISSION EN COURS" && contains "Voulez-vous voir les détails"; then
    pass "tutoriel passé : aucune consigne, journal de quêtes d'origine"
else fail "tutoriel passé" "code=$CODE"; fi

# Rappels : commande inconnue, verrouillée ou ratée.
run_in "$DATA/tuto2" "${TUTO_HEAD}xyzzy\nbruteforce localhost\nread\nquit\n" --new
n=$(printf '%s' "$OUT" | grep -c "Concentre-toi, rookie")
if [ "$CODE" -eq 0 ] && [ "$n" -eq 3 ] && contains "Ouvrir le journal de mission (quests)"; then
    pass "tutoriel : commande inconnue / verrouillée / ratée → ECHO-7 rappelle l'objectif"
else fail "rappels" "rappels=$n code=$CODE"; fi

# L'ordre compte : les commandes des étapes suivantes n'avancent pas l'étape en cours.
run_in "$DATA/tuto3" "${TUTO_HEAD}help\nscan\nstatus\nquit\n" --new
if [ "$CODE" -eq 0 ] && ! contains "Objectif accompli"; then
    pass "tutoriel : impossible de brûler les étapes dans le désordre"
else fail "étapes dans le désordre"; fi

# Parcours complet, avec une graine où « bruteforce localhost » réussit du premier coup.
FULL="${TUTO_HEAD}quests\nhelp\nscan\nstatus\nscan\nscan\nscan\nscan\nbruteforce localhost\nlaylow\n1\nstatus\nquit\n"
ok_seed=""
for seed in 1 2 3 4 5 6 7 8; do
    run_in "$DATA/tuto-full$seed" "$FULL" --new --seed "$seed"
    if contains "MISSION ACCOMPLIE"; then ok_seed=$seed; break; fi
done
steps=$(printf '%s' "$OUT" | grep -c "Objectif accompli")
if [ -n "$ok_seed" ] && [ "$steps" -eq 7 ] && contains "Pas mal du tout, Neo" && contains "[+100 crédits]" \
    && contains "[+10 réputation]" && contains "Réputation: 10" && contains "Niveau: 2 (Apprenti)"; then
    pass "tutoriel : les 7 étapes, mission accomplie, récompense versée (graine $ok_seed)"
else fail "parcours complet" "graine=${ok_seed:-aucune} étapes=$steps"; fi

# La récompense n'est versée qu'une fois, et la mission terminée se retrouve au menu suivant.
D="$DATA/tuto-full$ok_seed"
if [ -n "$ok_seed" ]; then
    run_in "$D" '1\nquests\n0\nstatus\nquit\n'
    if contains "Voulez-vous voir les détails" && ! contains "MISSION EN COURS" && ! contains "Content de te revoir" \
        && contains "Réputation: 10" && contains "Nom: Neo"; then
        pass "tutoriel terminé : rechargé sans reprise de mission, récompense non redoublée"
    else fail "reprise après tutoriel terminé"; fi
fi

# Tutoriel interrompu : ECHO-7 salue le joueur et reprend à l'étape en cours.
D="$DATA/tuto-resume"
run_in "$D" "${TUTO_HEAD}quests\nhelp\nquit\n" --new
run_in "$D" '1\nquit\n'
if [ "$CODE" -eq 0 ] && contains "Content de te revoir, Neo" && contains "'scan'"; then
    pass "tutoriel interrompu : à la reprise, ECHO-7 salue le joueur et redonne la consigne"
else fail "reprise du tutoriel" "code=$CODE"; fi
if file_has "$D/savegame.sav" "tutorial.step=3"; then pass "tutoriel : l'étape en cours est dans la sauvegarde"
else fail "étape sauvegardée"; fi

run_in "$DATA/tuto-en" 'Neo\n\n1\nxyzzy\nquit\n' --new --lang en
if contains "Focus, rookie" && contains "Open the mission log" && contains "Type 'quests'"; then
    pass "tutoriel en anglais"
else fail "tutoriel (en)"; fi

# --- Bilan -------------------------------------------------------------------

echo
echo "e2e : $passed réussi(s), $failed échec(s)"
[ "$failed" -eq 0 ]
