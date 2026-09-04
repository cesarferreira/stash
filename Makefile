.PHONY: all build build-release install install-release clean test check fmt lint run demo release

LEVEL ?= minor

# Default target
all: check build test

# Build debug version
build:
	cargo build

# Build release version
build-release:
	cargo build --release

# Install debug binary to ~/.cargo/bin
install:
	CARGO_INCREMENTAL=0 cargo install --path . --locked --bins --debug --force

# Install release binary to ~/.cargo/bin
install-release:
	CARGO_INCREMENTAL=0 cargo install --path . --locked --bins --force

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test

# Run clippy and check
check:
	cargo check
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Lint (check formatting)
lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

# Run with arguments (usage: make run ARGS="")
run:
	cargo run -- $(ARGS)

# Quick smoke install
demo: install
	@echo "=== stash ==="
	@echo "Launch with: stash"
	@echo "Toggle popup: hotkey from ~/.config/stash/stash.toml (default cmd+shift+v)"
	@echo "Quit: cmd+q"

# Bump version, regenerate CHANGELOG.md, tag, and push (requires cargo-release + git-cliff)
release:
	cargo release $(LEVEL) --execute --no-confirm
