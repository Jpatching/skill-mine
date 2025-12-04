# SKILL-MINE Development Commands
# Usage: make <target>

.PHONY: build test format lint clean check deploy

# Build all workspace packages
build:
	cargo build -p skill-api -p skill-program -p skill-cli

# Run Solana BPF tests
test:
	cargo test-sbf

# Format all code
format:
	cargo fmt --all

# Check formatting without modifying
format-check:
	cargo fmt --all -- --check

# Run clippy linter
lint:
	cargo clippy --all-targets -- -D warnings

# Full check (format + lint + build)
check: format-check lint build

# Clean build artifacts
clean:
	cargo clean

# Build for deployment
build-sbf:
	cargo build-sbf

# Deploy to devnet (requires KEYPAIR env var)
deploy:
	solana program deploy target/deploy/skill_program.so --program-id 3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU

# Show help
help:
	@echo "Available targets:"
	@echo "  build        - Build all workspace packages"
	@echo "  test         - Run Solana BPF tests"
	@echo "  format       - Format all code"
	@echo "  format-check - Check formatting"
	@echo "  lint         - Run clippy linter"
	@echo "  check        - Full check (format + lint + build)"
	@echo "  clean        - Clean build artifacts"
	@echo "  build-sbf    - Build for Solana deployment"
	@echo "  deploy       - Deploy to devnet"
