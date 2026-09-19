#!/bin/bash

# Script de nettoyage automatique du projet Neon Hack
# Supprime les fichiers temporaires et générés

echo "🧹 NETTOYAGE AUTOMATIQUE DU PROJET NEON HACK"
echo "=============================================="

# Supprimer les fichiers objets
echo "📁 Suppression des fichiers objets (.o)..."
find . -name "*.o" -type f -delete
echo "✅ Fichiers .o supprimés"

# Supprimer les fichiers de sauvegarde temporaires
echo "📁 Suppression des fichiers de sauvegarde..."
find . -name "*~" -type f -delete
find . -name "*.bak" -type f -delete
find . -name "*.tmp" -type f -delete
echo "✅ Fichiers de sauvegarde supprimés"

# Supprimer les fichiers de logs temporaires
echo "📁 Suppression des logs temporaires..."
find . -name "*.log" -type f -delete
echo "✅ Logs temporaires supprimés"

# Nettoyer les fichiers cachés système (si présents)
echo "📁 Nettoyage des fichiers cachés système..."
find . -name ".DS_Store" -type f -delete 2>/dev/null
find . -name "Thumbs.db" -type f -delete 2>/dev/null
echo "✅ Fichiers système nettoyés"

echo ""
echo "🎉 NETTOYAGE TERMINÉ !"
echo "Le projet est maintenant propre et organisé."
echo ""
echo "📊 Structure finale du projet :"
echo "├── neon_hack           (exécutable principal)"
echo "├── neon_hack.c         (code source principal)"
echo "├── Makefile            (configuration build)"
echo "├── README.md           (documentation)"
echo "├── src/game/           (modules du jeu)"
echo "├── data/               (données du jeu)"
echo "├── tests/              (tests essentiels)"
echo "└── demo_*.sh           (scripts de démonstration)"
