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

# Nouveau code (socle) : aucun warning toléré. Le code d'origine (neon_hack.c,
# src/game/) n'est pas encore soumis à ces règles, il sera réécrit.
STRICT  := -Wpedantic -Wshadow -Wconversion -Werror

LEGACY_SRC := neon_hack.c $(wildcard src/game/*.c)
CORE_SRC   := $(wildcard src/core/*.c src/ui/*.c src/i18n/*.c)
TEST_SRC   := $(wildcard tests/unit/test_*.c)

LEGACY_OBJ := $(LEGACY_SRC:%.c=$(BUILD)/obj/%.o)
CORE_OBJ   := $(CORE_SRC:%.c=$(BUILD)/obj/%.o)
TEST_BIN   := $(TEST_SRC:tests/unit/%.c=$(BUILD)/tests/%)

all: $(BIN)

$(BIN): $(LEGACY_OBJ) $(CORE_OBJ)
	$(CC) $^ $(LDFLAGS) -o $@

$(BUILD)/obj/%.o: %.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

$(CORE_OBJ): override CFLAGS += $(STRICT)

$(BUILD)/tests/%: tests/unit/%.c tests/unit/nh_test.h $(CORE_OBJ)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(STRICT) $< $(CORE_OBJ) $(LDFLAGS) -o $@

run: $(BIN)
	./$(BIN)

unit: $(TEST_BIN)
	@status=0; for t in $(TEST_BIN); do ./$$t || status=1; done; exit $$status

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

-include $(LEGACY_OBJ:.o=.d) $(CORE_OBJ:.o=.d)

.PHONY: all run unit e2e test asan clean rebuild help
