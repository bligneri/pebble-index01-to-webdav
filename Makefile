.PHONY: build test run key secret sealed-secret

# kubeseal flags, e.g. KUBESEAL_ARGS="--controller-namespace kube-system --controller-name sealed-secrets"
KUBESEAL_ARGS ?=

build:
	cargo build --release

test:
	cargo test

run:
	CONFIG_PATH=config.example.yaml cargo run

# Random signing secret to paste in the Pebble app.
key:
	@openssl rand -hex 32

# Plain Kubernetes Secret on stdout (NAMESPACE, SECRET_NAME overridable).
secret:
	@sh scripts/secret.sh

# Same values sealed with kubeseal, never written in clear.
sealed-secret:
	@mkdir -p deploy
	@sh scripts/secret.sh | kubeseal --format yaml $(KUBESEAL_ARGS) > deploy/sealed-secret.yaml
	@echo "wrote deploy/sealed-secret.yaml" >&2
