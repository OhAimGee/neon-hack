#!/bin/bash

# Test des nouvelles fonctionnalités avancées de Neon Hack
echo "=== TEST DES FONCTIONNALITÉS AVANCÉES ==="
echo "Date: $(date)"
echo

# Fonction pour tester une commande
test_command() {
    echo "Test: $1"
    echo "$2" | timeout 5s ../neon_hack
    echo "----------------------------------------"
}

# Test 1: Vérification du statut initial avec nouvelles métriques
echo "1. Test du statut étendu"
echo -e "TestUser\nstatus\nquit" | timeout 10s ../neon_hack
echo "========================================="

# Test 2: Test progression rapide vers niveau 3 pour débloquer nouvelles commandes
echo "2. Test progression et déblocage nouvelles commandes"
echo -e "HackerPro\nscan\nbruteforce localhost\nscan\nbruteforce corp-server-01\nscan\nbruteforce nexus-mainframe\nstatus\nhelp\nquit" | timeout 15s ../neon_hack
echo "========================================="

# Test 3: Test nouvelles commandes si niveau suffisant
echo "3. Test commandes avancées (nécessite niveau élevé)"
echo -e "AdvancedHacker\nscan\nbruteforce localhost\nbruteforce corp-server-01\nbruteforce nexus-mainframe\nbruteforce localhost\nbruteforce corp-server-01\nbackdoor localhost\ntraceroute corp-server-01\nstealth\nstatus\nquit" | timeout 20s ../neon_hack
echo "========================================="

# Test 4: Test des nouvelles commandes d'aide
echo "4. Test affichage aide avec nouvelles commandes"
echo -e "Helper\nhelp\nquit" | timeout 10s ../neon_hack
echo "========================================="

echo "Tests terminés !"
echo "Vérifiez que :"
echo "- Le statut affiche crédits, réputation, furtivité, équipement"
echo "- Les nouvelles commandes se débloquent progressivement"
echo "- Les backdoors et traceroute fonctionnent"
echo "- Le mode furtif s'active correctement"
