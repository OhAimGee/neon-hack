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

# run <entrée> [options...] : met la sortie (stdout+stderr, sans couleurs, bornée)
# dans $OUT et le code de retour du jeu dans $CODE. Le français est la langue par
# défaut des tests (indépendant de $LANG) ; un test peut la surcharger avec --lang.
#
# La sortie est bornée par `head -c` : si le jeu se remet à boucler à l'infini,
# il reçoit SIGPIPE (code 141) au lieu de remplir le disque.
run() {
    local input=$1 tmp
    shift
    tmp=$(mktemp)
    printf '%b' "$input" | timeout "$LIMIT" "$BIN" --lang fr "$@" 2>&1 | head -c "$MAX_BYTES" >"$tmp"
    CODE=${PIPESTATUS[1]}
    OUT=$(strip_ansi <"$tmp")
    rm -f "$tmp"
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
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ]; then
    pass "entrée vide : le jeu s'arrête proprement (${#OUT} octets)"
else fail "entrée vide" "code=$CODE taille=${#OUT} (124 = boucle infinie)"; fi

run 'Testeur\n' --fast --lang fr
if [ "$CODE" -eq 0 ] && [ "${#OUT}" -lt 10000 ] && contains "Entrée fermée"; then
    pass "EOF après le nom : fin de session annoncée"
else fail "EOF après le nom" "code=$CODE taille=${#OUT}"; fi

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
if contains "Nom: Anonymous"; then
    pass "nom vide : remplacé par « Anonymous »"
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
raw=$(printf 'T\nhelp\nstatus\nquit\n' | "$BIN" --fast --lang fr --no-color 2>&1 \
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

# --- Bilan -------------------------------------------------------------------

echo
echo "e2e : $passed réussi(s), $failed échec(s)"
[ "$failed" -eq 0 ]
