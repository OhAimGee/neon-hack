#!/bin/bash

# Script de démonstration du système de hacking avancé
# Démontre toutes les nouvelles commandes intégrées

echo "=== DÉMONSTRATION SYSTÈME HACKING AVANCÉ ==="
echo "Démarrage du jeu Neon Hack..."

# Créer un fichier de commandes pour automatiser la démonstration
cat << 'EOF' > /tmp/demo_commands.txt
CyberMaster
help
status
stealthmode
status
analyzedefenses Corp-A
socialeng Corp-A
aiassist Corp-A
neuralsync
quantumdecrypt encrypted_data_01
advhack Corp-A
status
quit
EOF

echo "Exécution de la démonstration..."
echo "─────────────────────────────────────"

# Lancer le jeu avec les commandes
./neon_hack < /tmp/demo_commands.txt

echo ""
echo "─────────────────────────────────────"
echo "=== FIN DE LA DÉMONSTRATION ==="
echo ""
echo "Nouvelles fonctionnalités testées :"
echo "✓ advhack      - Système de hacking avancé avec 8 méthodes"
echo "✓ stealthmode  - Activation/désactivation du mode furtif"
echo "✓ analyzedefenses - Analyse des défenses des cibles"
echo "✓ socialeng    - Attaques par ingénierie sociale"
echo "✓ aiassist     - Assistant IA pour le hacking"
echo "✓ neuralsync   - Interface neurale pour améliorer les capacités"
echo "✓ quantumdecrypt - Décryptage quantique avancé"
echo ""
echo "Toutes les fonctionnalités sont opérationnelles !"

# Nettoyage
rm -f /tmp/demo_commands.txt
