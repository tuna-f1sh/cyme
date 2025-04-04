PROJECT_NAME := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')
VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
OS := $(shell uname)

RSRCS += $(wildcard src/*.rs src/**/*.rs)
DOCS = $(AUTOCOMPLETE_FILES) doc/$(PROJECT_NAME).1  doc/cyme_example_config.json
AUTOCOMPLETE_FILES = doc/_$(PROJECT_NAME) doc/$(PROJECT_NAME).bash doc/$(PROJECT_NAME).fish doc/_$(PROJECT_NAME).ps1

# ?= allows overriding from command line with 'cross'
CARGO_CMD ?= cargo
CARGO_TARGET_DIR ?= target
PACKAGE_DIR ?= $(CARGO_TARGET_DIR)/packages
CARGO_RELEASE_FLAGS += --locked --release

ifeq ($(TARGET),)
	PACKAGE_BASE := $(PROJECT_NAME)-v$(VERSION)-$(OS)
	TARGET_DIR := $(CARGO_TARGET_DIR)
else
	PACKAGE_BASE := $(PROJECT_NAME)-v$(VERSION)-$(TARGET)
	TARGET_DIR := $(CARGO_TARGET_DIR)/$(TARGET)
	CARGO_RELEASE_FLAGS += --target $(TARGET)
endif
RELEASE_BIN := $(TARGET_DIR)/release/$(PROJECT_NAME)

ifeq ($(findstring windows,$(TARGET)),windows)
	ARCHIVE_EXT := zip
else
	ARCHIVE_EXT := tar.gz
endif
ARCHIVE := $(PACKAGE_DIR)/$(PACKAGE_BASE).$(ARCHIVE_EXT)

ifeq ($(OS), Darwin)
	PREFIX ?= /usr/local
else
	PREFIX ?= /usr
endif

BIN_PATH ?= $(PREFIX)/bin
BASH_COMPLETION_PATH ?= $(PREFIX)/share/bash-completion/completions
ZSH_COMPLETION_PATH ?= $(PREFIX)/share/zsh/site-functions
MAN_PAGE_PATH ?= $(PREFIX)/share/man/man1

.PHONY: release install generated enter_version new_version test package

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
	$(CARGO_CMD) update

new_version: enter_version generated

test:
	$(CARGO_CMD) test $(CARGO_TEST_FLAGS)

package: $(ARCHIVE)
	@echo "$(ARCHIVE)"

$(DOCS): Cargo.toml $(RSRCS)
	@echo "Generating docs for $(PROJECT_NAME) $(VERSION)"
	$(CARGO_CMD) run --locked --release -F=cli_generate -- --gen

$(RELEASE_BIN): Cargo.lock $(RSRCS)
ifeq ($(TARGET),universal-apple-darwin)
	cargo build --target aarch64-apple-darwin $(CARGO_RELEASE_FLAGS)
	cargo build --target x86_64-apple-darwin $(CARGO_RELEASE_FLAGS)
	mkdir -p $(shell dirname $(RELEASE_BIN))
	lipo -create -output $(RELEASE_BIN) \
	  $(CARGO_TARGET_DIR)/aarch64-apple-darwin/release/$(PROJECT_NAME) \
	  $(CARGO_TARGET_DIR)/x86_64-apple-darwin/release/$(PROJECT_NAME)
else
	$(CARGO_CMD) build --target $(TARGET) $(CARGO_RELEASE_FLAGS)
endif
	@echo "$(RELEASE_BIN)"

$(ARCHIVE): $(RELEASE_BIN) README.md LICENSE CHANGELOG.md $(DOCS)
	mkdir -p $(PACKAGE_DIR)/$(PACKAGE_BASE)
	cp $(RELEASE_BIN) $(PACKAGE_DIR)/$(PACKAGE_BASE)/
	cp README.md LICENSE CHANGELOG.md $(PACKAGE_DIR)/$(PACKAGE_BASE)/
	cp 'doc/$(PROJECT_NAME).1' $(PACKAGE_DIR)/$(PACKAGE_BASE)/
	mkdir -p $(PACKAGE_DIR)/$(PACKAGE_BASE)/autocomplete
	cp $(AUTOCOMPLETE_FILES) $(PACKAGE_DIR)/$(PACKAGE_BASE)/autocomplete/
ifeq ($(ARCHIVE_EXT),zip)
	cd $(PACKAGE_DIR) && zip -r $(PACKAGE_BASE).zip $(PACKAGE_BASE)
else
	cd $(PACKAGE_DIR) && tar czf $(PACKAGE_BASE).tar.gz $(PACKAGE_BASE)
endif
	rm -rf $(PACKAGE_DIR)/$(PACKAGE_BASE)

%.deb:
	cargo deb --target $(TARGET) --no-strip --no-build --output $@
