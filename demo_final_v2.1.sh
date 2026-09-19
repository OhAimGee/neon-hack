#!/bin/bash

# Script de démonstration finale - Neon Hack v2.1
# Présentation complète du jeu finalisé

clear

echo "═══════════════════════════════════════════════════════════════════"
echo "      🔥 NEON HACK v2.1 - DÉMONSTRATION FINALE 🔥"
echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo "🎮 Bienvenue dans la version finale de Neon Hack !"
echo ""
echo "✨ NOUVEAUTÉS v2.1 :"
echo "   🛒 Boutique cyberpunk de R4Z0R"
echo "   🕳️  Système de planque (laylow)"
echo "   💰 Économie de crédits"
echo "   🎨 Interface graphique corrigée"
echo "   🔧 Architecture modulaire"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""

# Vérification de la compilation
echo "🔧 Vérification de la compilation..."
if [ -f "./neon_hack" ]; then
    echo "✅ Jeu compilé et prêt à jouer"
else
    echo "⚙️  Compilation en cours..."
    make > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo "✅ Compilation réussie !"
    else
        echo "❌ Erreur de compilation"
        exit 1
    fi
fi

echo ""
echo "🎯 COMMANDES DISPONIBLES :"
echo ""
echo "📋 COMMANDES DE BASE :"
echo "   • help     - Aide complète"
echo "   • status   - Votre statut (niveau, XP, alerte, crédits)"
echo "   • scan     - Scanner le réseau"
echo "   • clear    - Effacer l'écran"
echo "   • quit     - Quitter le jeu"
echo ""
echo "💻 COMMANDES DE HACKING :"
echo "   • bruteforce <cible>  - Attaque par force brute"
echo "   • decrypt <message>   - Décrypter des messages"
echo "   • exploit <cible>     - Exploiter des vulnérabilités"
echo ""
echo "🆕 NOUVELLES COMMANDES v2.1 :"
echo "   • shop     - Marché noir de R4Z0R"
echo "   • laylow   - Se cacher (réduit l'alerte)"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo "🛒 APERÇU DE LA BOUTIQUE CYBERPUNK :"
echo ""
echo "   Vendeur : R4Z0R - Underground Market, Sector 7"
echo ""
echo "   📦 OBJETS DISPONIBLES :"
echo "   • Stealth Module v2.0      - 150 crédits"
echo "   • Ghost Protocol           - 80 crédits"
echo "   • Malware Arsenal          - 120 crédits"
echo "   • Proxy Chain Pro          - 100 crédits"
echo "   • Quantum Encryption Key   - 200 crédits"
echo "   • Neural Assistant v3.1    - 500 crédits"
echo "   • Quantum Processing Chip  - 800 crédits"
echo "   • Street Cred Booster      - 75 crédits"
echo "   • Neural Accelerator       - 90 crédits"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo "🎮 GUIDE DE DÉMARRAGE RAPIDE :"
echo ""
echo "1. Lancez le jeu : ./neon_hack"
echo "2. Entrez votre nom de hacker"
echo "3. Tapez 'help' pour l'aide complète"
echo "4. Commencez par 'scan' puis 'bruteforce localhost'"
echo "5. Utilisez 'status' pour suivre votre progression"
echo "6. Visitez la boutique avec 'shop'"
echo "7. Gérez votre alerte avec 'laylow'"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""

# Proposer de lancer le jeu ou juste montrer l'aide
echo "🚀 Que voulez-vous faire ?"
echo ""
echo "1. Lancer le jeu maintenant"
echo "2. Voir l'aide du jeu uniquement"
echo "3. Quitter cette démo"
echo ""
read -p "Votre choix (1-3): " choice

case $choice in
    1)
        echo ""
        echo "🎮 Lancement de Neon Hack v2.1..."
        echo "═══════════════════════════════════════════════════════════════════"
        sleep 2
        ./neon_hack
        ;;
    2)
        echo ""
        echo "📋 Affichage de l'aide..."
        echo "═══════════════════════════════════════════════════════════════════"
        sleep 1
        echo "DemoUser" | ./neon_hack | head -50
        ;;
    3)
        echo ""
        echo "👋 Merci d'avoir testé Neon Hack v2.1 !"
        echo "   Pour jouer plus tard : ./neon_hack"
        echo ""
        ;;
    *)
        echo ""
        echo "👋 Au revoir ! Pour jouer : ./neon_hack"
        echo ""
        ;;
esac

echo "═══════════════════════════════════════════════════════════════════"
echo "🔥 NEON HACK v2.1 - Dans l'ombre des néons, seuls les codes survivent..."
echo "═══════════════════════════════════════════════════════════════════"
