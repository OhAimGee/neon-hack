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

# --- Bilan -------------------------------------------------------------------

echo
echo "e2e : $passed réussi(s), $failed échec(s)"
[ "$failed" -eq 0 ]
