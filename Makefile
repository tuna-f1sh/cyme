PROJECT_NAME := $(shell grep -m 1 name Cargo.toml | cut -d '"' -f 2)
RELEASE_BIN := $(CARGO_TARGET_DIR)/release/$(PROJECT_NAME)
VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

RSRCS += $(wildcard src/*.rs src/**/*.rs)
CARGO_TARGET_DIR ?= target
CARGO_RELEASE_FLAGS += --locked --release
DOCS = doc/_$(PROJECT_NAME) doc/$(PROJECT_NAME).1 doc/$(PROJECT_NAME).bash doc/$(PROJECT_NAME).fish doc/$(PROJECT_NAME).ps1 doc/cyme_example_config.json

OS := $(shell uname)

ifeq ($(OS), Darwin)
	PREFIX ?= /usr/local
else
	PREFIX ?= /usr
endif
BIN_PATH ?= $(PREFIX)/bin
BASH_COMPLETION_PATH ?= $(PREFIX)/share/bash-completion/completions
ZSH_COMPLETION_PATH ?= $(PREFIX)/share/zsh/site-functions
MAN_PAGE_PATH ?= $(PREFIX)/share/man/man1

.PHONY: release install generated enter_version new_version test

release: $(RELEASE_BIN)

install: release
	@echo "Installing $(PROJECT_NAME) $(VERSION)"
	install -Dm755 "$(RELEASE_BIN)" "$(DESTDIR)$(BIN_PATH)/$(PROJECT_NAME)"
	install -Dm644 ./doc/$(PROJECT_NAME).1 "$(DESTDIR)$(MAN_PAGE_PATH)/$(PROJECT_NAME).1"
	@if [ -d "$(DESTDIR)$(BASH_COMPLETION_PATH)" ]; then \
		install -vDm0644 ./doc/$(PROJECT_NAME).bash "$(DESTDIR)$(BASH_COMPLETION_PATH)/$(PROJECT_NAME).bash"; \
	fi
	@if [ -d "$(DESTDIR)$(ZSH_COMPLETION_PATH)" ]; then \
		install -vDm0644 ./doc/_$(PROJECT_NAME) "$(DESTDIR)$(ZSH_COMPLETION_PATH)/_$(PROJECT_NAME)"; \
	fi

generated: $(DOCS)

enter_version:
	@echo "Current version: $(VERSION)"
	@echo "Enter new version: "
	@read new_version; \
	sed -i "s/^version = .*/version = \"$$new_version\"/" Cargo.toml
	# update because Cargo.lock references self for tests
	cargo update

new_version: enter_version generated

test:
	cargo test $(CARGO_TEST_FLAGS)

$(RELEASE_BIN): Cargo.lock $(RSRCS)
	@echo "Building version $(PROJECT_NAME) $(VERSION)"
	cargo build $(CARGO_RELEASE_FLAGS)

$(DOCS): Cargo.toml $(RSRCS)
	@echo "Generating docs for $(PROJECT_NAME) $(VERSION)"
	cargo run -F=cli_generate -- --gen
