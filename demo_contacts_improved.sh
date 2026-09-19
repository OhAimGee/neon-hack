#!/bin/bash

# Démonstration du système de contacts amélioré
# Montre les différentes façons d'interagir avec les contacts

echo "🎮 DÉMONSTRATION SYSTÈME DE CONTACTS AMÉLIORÉ"
echo "============================================="
echo ""
echo "Fonctionnalités testées :"
echo "• Menu contacts interactif avec numéros clairs"
echo "• Commande directe 'contact <numéro>'"
echo "• Commande directe 'contact <nom>'"
echo "• Affichage des actions disponibles pour chaque contact"
echo ""
echo "═══════════════════════════════════════════"
echo "Test 1: Menu contacts traditionnel"
echo "═══════════════════════════════════════════"

# Test du menu contacts traditionnel
cat << 'EOF' > /tmp/demo_contacts_1.txt
ContactDemo1
contacts
1
5
quit
EOF

echo "Commandes: contacts -> 1 -> 5 (terminer conversation) -> quit"
echo ""
./neon_hack < /tmp/demo_contacts_1.txt

echo ""
echo "═══════════════════════════════════════════"
echo "Test 2: Commande directe par numéro"
echo "═══════════════════════════════════════════"

# Test de la commande directe par numéro
cat << 'EOF' > /tmp/demo_contacts_2.txt
ContactDemo2
contact 1
1
5
quit
EOF

echo "Commandes: contact 1 -> 1 (demander conseils) -> 5 (terminer) -> quit"
echo ""
./neon_hack < /tmp/demo_contacts_2.txt

echo ""
echo "═══════════════════════════════════════════"
echo "Test 3: Commande directe par nom"
echo "═══════════════════════════════════════════"

# Test de la commande directe par nom
cat << 'EOF' > /tmp/demo_contacts_3.txt
ContactDemo3
contact ECHO-7
2
5
quit
EOF

echo "Commandes: contact ECHO-7 -> 2 (signaler progression) -> 5 (terminer) -> quit"
echo ""
./neon_hack < /tmp/demo_contacts_3.txt

echo ""
echo "═══════════════════════════════════════════"
echo "Test 4: Aide pour les commandes"
echo "═══════════════════════════════════════════"

# Test des messages d'aide
cat << 'EOF' > /tmp/demo_contacts_4.txt
ContactDemo4
contact
help
quit
EOF

echo "Commandes: contact (sans argument pour voir l'aide) -> help (pour voir les commandes) -> quit"
echo ""
./neon_hack < /tmp/demo_contacts_4.txt

echo ""
echo "🎉 DÉMONSTRATION TERMINÉE !"
echo ""
echo "✅ AMÉLIORATIONS APPORTÉES :"
echo "   • Numéros [1], [2], etc. clairement affichés dans la liste"
echo "   • Actions disponibles montrées pour chaque contact"
echo "   • Commande 'contact <numéro>' pour interaction directe"
echo "   • Commande 'contact <nom>' pour interaction par nom"
echo "   • Messages d'aide améliorés et clairs"
echo "   • Instructions contextuelles dans chaque menu"
echo ""
echo "📱 UTILISATION :"
echo "   • Depuis le menu: tapez 'contacts' puis le numéro"
echo "   • Depuis le terminal: tapez 'contact 1' ou 'contact ECHO-7'"
echo "   • Toujours disponible: 'help' pour voir toutes les commandes"

# Nettoyage
rm -f /tmp/demo_contacts_*.txt
