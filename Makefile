.PHONY: build test lint image run key secret sealed-secret

# kubeseal flags, e.g. KUBESEAL_ARGS="--controller-namespace kube-system --controller-name sealed-secrets"
KUBESEAL_ARGS ?=
NAMESPACE ?= pebble
SECRET_NAME ?= pebble-index01-to-webdav
SEALED_OUT ?= deploy/sealed-secret.yaml

build:
	cargo build --release

test:
	cargo test

lint:
	cargo fmt --check
	cargo clippy --all-targets

image:
	docker build -t pebble-index01-to-webdav .

run:
	CONFIG_PATH=config.example.yaml cargo run

# Random signing secret to paste in the Pebble app.
key:
	@openssl rand -hex 32

# Plain Kubernetes Secret on stdout (NAMESPACE, SECRET_NAME overridable).
secret:
	@NAMESPACE=$(NAMESPACE) SECRET_NAME=$(SECRET_NAME) sh scripts/secret.sh

# Same values sealed with kubeseal, never written in clear.
sealed-secret:
	@mkdir -p $(dir $(SEALED_OUT))
	@NAMESPACE=$(NAMESPACE) SECRET_NAME=$(SECRET_NAME) sh scripts/secret.sh | kubeseal --format yaml $(KUBESEAL_ARGS) > $(SEALED_OUT).tmp || { rm -f $(SEALED_OUT).tmp; exit 1; }
	@mv $(SEALED_OUT).tmp $(SEALED_OUT)
	@echo "wrote $(SEALED_OUT)" >&2
