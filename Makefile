# Makefile de Neon Hack
#
#   make          compile le jeu (./neon_hack)
#   make run      compile puis lance
#   make test     tests unitaires + tests de bout en bout
#   make asan     idem, compilé avec AddressSanitizer + UBSan
#   make clean    supprime les fichiers générés

CC      ?= gcc
VERSION ?= 0.1.0-dev+$(shell git rev-parse --short HEAD 2>/dev/null || echo nogit)

# Répertoire de compilation et exécutable (surchargés par la cible `asan`)
BUILD   ?= build/default
BIN     ?= neon_hack

CFLAGS  ?= -g -O2
override CFLAGS += -std=c11 -D_DEFAULT_SOURCE -Wall -Wextra \
                   -DNH_VERSION=\"$(VERSION)\" -MMD -MP

# Code neuf : aucun warning toléré. Le code d'origine encore en place
# (src/game/cmd_*.c, game.c, modules historiques) n'est pas soumis à ces règles :
# il sera réécrit phase par phase, pas corrigé.
STRICT  := -Wpedantic -Wshadow -Wconversion -Werror

MAIN_SRC := src/main.c
GAME_SRC := $(wildcard src/game/*.c)
CORE_SRC := $(wildcard src/core/*.c src/ui/*.c src/i18n/*.c)
TEST_SRC := $(wildcard tests/unit/test_*.c)

MAIN_OBJ := $(MAIN_SRC:%.c=$(BUILD)/obj/%.o)
GAME_OBJ := $(GAME_SRC:%.c=$(BUILD)/obj/%.o)
CORE_OBJ := $(CORE_SRC:%.c=$(BUILD)/obj/%.o)
TEST_BIN := $(TEST_SRC:tests/unit/%.c=$(BUILD)/tests/%)

# Fichiers du dossier game/ déjà écrits au nouveau standard
STRICT_OBJ := $(CORE_OBJ) $(MAIN_OBJ) $(BUILD)/obj/src/game/commands.o $(BUILD)/obj/src/game/alert.o

all: $(BIN)

$(BIN): $(MAIN_OBJ) $(GAME_OBJ) $(CORE_OBJ)
	$(CC) $^ $(LDFLAGS) -o $@

$(BUILD)/obj/%.o: %.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

$(STRICT_OBJ): override CFLAGS += $(STRICT)

# NH_VERSION est passé par -D : make ne voit pas ce changement (nouveau commit) et
# garderait un config.o périmé. Le fichier-témoin n'est réécrit que si la version change.
$(BUILD)/version.stamp: FORCE
	@mkdir -p $(dir $@)
	@echo '$(VERSION)' | cmp -s - $@ || echo '$(VERSION)' > $@
$(BUILD)/obj/src/core/config.o: $(BUILD)/version.stamp
$(BUILD)/tests/test_config: $(BUILD)/version.stamp
FORCE:

$(BUILD)/tests/%: tests/unit/%.c tests/unit/nh_test.h tests/unit/nh_capture.h $(CORE_OBJ) $(GAME_OBJ)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(STRICT) $< $(CORE_OBJ) $(GAME_OBJ) $(LDFLAGS) -o $@

run: $(BIN)
	./$(BIN)

unit: $(TEST_BIN)
	@status=0; for t in $(TEST_BIN); do ./$$t </dev/null || status=1; done; exit $$status

e2e: $(BIN)
	NEON_HACK_BIN=./$(BIN) tests/e2e/run.sh

test: unit e2e

asan:
	$(MAKE) BUILD=build/asan BIN=build/asan/neon_hack \
	  CFLAGS="-g -O1 -fsanitize=address,undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer" \
	  LDFLAGS="-fsanitize=address,undefined" test

clean:
	rm -rf build $(BIN)

rebuild: clean all

help:
	@echo "make          compile le jeu (./neon_hack)"
	@echo "make run      compile puis lance"
	@echo "make test     tests unitaires + bout en bout"
	@echo "make asan     tests avec ASan + UBSan"
	@echo "make clean    nettoie"

-include $(MAIN_OBJ:.o=.d) $(GAME_OBJ:.o=.d) $(CORE_OBJ:.o=.d)

.PHONY: all run unit e2e test asan clean rebuild help
