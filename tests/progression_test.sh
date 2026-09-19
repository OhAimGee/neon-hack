#!/bin/bash

echo "=== TEST DE PROGRESSION COMPLÈTE ==="
echo ""

# Générer 21 scans pour garantir le passage au niveau 2 (105 EXP)
commands="TestHacker\n"
for i in {1..21}; do
    commands+="scan localhost\n"
done
commands+="status\nhelp\nbruteforce localhost\ndecrypt WKLV#LV#D#WHVW\nstatus\nquit"

echo -e "$commands" | ./neon_hack

echo ""
echo "=== Test de progression terminé ==="
