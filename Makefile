# Makefile pour Neon Hack RPG
CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -g -D_DEFAULT_SOURCE
TARGET = neon_hack
MAIN_SOURCE = neon_hack.c

# Sources des modules
GAME_SOURCES = src/game/shop.c src/game/alert_system.c src/game/quest_system.c src/game/contacts.c src/game/advanced_hacking.c
SOURCES = $(MAIN_SOURCE) $(GAME_SOURCES)
OBJECTS = $(SOURCES:.c=.o)

# Règle par défaut
all: $(TARGET)

# Compiler l'exécutable avec les modules
$(TARGET): $(OBJECTS)
	$(CC) $(CFLAGS) $(OBJECTS) -o $(TARGET)

# Règle pour compiler les fichiers objets
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Nettoyer les fichiers compilés
clean:
	rm -f $(TARGET) $(OBJECTS)

# Recompiler entièrement
rebuild: clean all

# Lancer le jeu
run: $(TARGET)
	./$(TARGET)

# Aide
help:
	@echo "Commandes disponibles :"
	@echo "  make          - Compile le jeu"
	@echo "  make run      - Compile et lance le jeu"
	@echo "  make clean    - Nettoie les fichiers compilés"
	@echo "  make rebuild  - Recompile entièrement"
	@echo "  make help     - Affiche cette aide"

.PHONY: all clean rebuild run help
