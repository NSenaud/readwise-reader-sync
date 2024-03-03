VERSION := `git describe --exact-match --tags 2> /dev/null || git rev-parse --short HEAD`

all: build push

build:
    cargo sqlx prepare
    podman build -t readwise-sync:{{VERSION}} .
    docker tag readwise-sync:{{VERSION}} rg.fr-par.scw.cloud/tooling/readwise-sync:{{VERSION}}

push:
    docker push rg.fr-par.scw.cloud/tooling/readwise-sync:{{VERSION}}

dev-install:
    cargo install --version=^0.7 sqlx-cli --no-default-features --features postgres
