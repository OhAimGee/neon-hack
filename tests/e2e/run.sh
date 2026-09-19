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
# dans $OUT et le code de retour du jeu dans $CODE.
#
# La sortie est bornée par `head -c` : si le jeu se remet à boucler à l'infini,
# il reçoit SIGPIPE (code 141) au lieu de remplir le disque.
run() {
    local input=$1 tmp
    shift
    tmp=$(mktemp)
    printf '%b' "$input" | timeout "$LIMIT" "$BIN" "$@" 2>&1 | head -c "$MAX_BYTES" >"$tmp"
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
