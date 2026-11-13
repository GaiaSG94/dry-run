.DEFAULT_GOAL := help

CARGO ?= cargo
RUSTFLAGS ?= -C opt-level=3

.PHONY: help build build-contracts build-tools test demo attack clean fmt

help:
	@echo "Dry Run — Cibles disponibles :"
	@echo "  make build          Compile les contrats WASM et les outils"
	@echo "  make test           Exécute tous les tests unitaires et invariants"
	@echo "  make demo           Déroule la démo complète de bout en bout"
	@echo "  make attack         Joue le corpus des 7 contrats piégés avec et sans pare-feu"
	@echo "  make clean          Nettoie les artefacts de compilation"

build: build-contracts build-tools

build-contracts:
	@echo "==> Compilation des contrats WASM..."
	@$(CARGO) build --target wasm32-unknown-unknown --release -p vault
	@$(CARGO) build --target wasm32-unknown-unknown --release -p good-dex -p test-token -p test-nft
	@$(CARGO) build --target wasm32-unknown-unknown --release -p trap-drainer -p trap-skimmer -p trap-half-step -p trap-liar -p trap-approver -p trap-shapeshifter -p trap-two-face

build-tools:
	@echo "==> Compilation des outils (core, sim, gate)..."
	@$(CARGO) build --release -p dryrun-core -p dryrun-sim -p dryrun-gate

test:
	@echo "==> Exécution de la suite de tests..."
	@$(CARGO) test --workspace

lint:
	@echo "==> Vérification format et clippy..."
	@cargo fmt --all --check
	@$(CARGO) clippy --workspace --all-targets -- -D warnings

demo: build
	@echo "==> Lancement de la démonstration Dry Run..."
	@$(CARGO) run --release -p dryrun-gate -- demo

attack: build
	@echo "==> Lancement du corpus d'attaque..."
	@$(CARGO) run --release -p dryrun-gate -- attack

sim: build
	@echo "==> Lancement du banc de test automatisé (200 robots)..."
	@$(CARGO) run --release -p dryrun-gate -- sim 42

mcp: build
	@echo "==> Démarrage du serveur MCP Dry Run..."
	@$(CARGO) run --release -p dryrun-gate -- mcp

demo-testnet:
	@echo "==> Vérification / déploiement NEAR Testnet..."
	@./scripts/demo-testnet.sh

clean:
	@$(CARGO) clean

fmt:
	@cargo fmt --all
