#!/bin/bash

echo "=== TEST COMPLET AVEC PROGRESSION ==="
echo ""

# Simuler une progression complète 
# Besoin de 20 EXP pour le niveau 2 (4 scans)
echo -e "Hacker\nscan localhost\nscan localhost\nscan localhost\nscan localhost\nstatus\nhelp\nbruteforce localhost\nstatus\nquit" | ./neon_hack

echo ""
echo "=== Test complet terminé ==="
