PROJECT_NAME := $(shell grep -m 1 name Cargo.toml | cut -d '"' -f 2)
CARGO_TARGET_DIR ?= target
RELEASE_BIN = $(CARGO_TARGET_DIR)/release/$(PROJECT_NAME)
VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

RSRCS += $(wildcard src/*.rs src/**/*.rs)
DOCS = doc/_$(PROJECT_NAME) doc/$(PROJECT_NAME).1 doc/$(PROJECT_NAME).bash doc/cyme_example_config.json

.PHONY: release install generated enter_version new_version

release: $(RELEASE_BIN)

install:
	$(shell ./install.sh)

generated: $(DOCS)

enter_version:
	@echo "Current version: $(VERSION)"
	@echo "Enter new version: "
	@read new_version; \
	sed -i "s/^version = .*/version = \"$$new_version\"/" Cargo.toml

new_version: enter_version generated

$(RELEASE_BIN): Cargo.toml $(RSRCS)
	@echo "Building version $(PROJECT_NAME) $(VERSION)"
	cargo build --release

$(DOCS): Cargo.toml $(RSRCS)
	@echo "Generating docs for $(PROJECT_NAME) $(VERSION)"
	cargo run -F=cli_generate -- --gen
