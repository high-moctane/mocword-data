DOCKER_COMPOSE_DOWNLOAD := docker-compose \
		-f docker-compose.download.yml \
		-p mocword_download

DOCKER_COMPOSE_TEST := docker-compose \
		-f docker-compose.test.yml \
		-p mocword_test


.PHONY: build
build:


.PHONY: build-download
build-download:
	$(DOCKER_COMPOSE_DOWNLOAD) build


.PHONY: clean
clean:
	$(DOCKER_COMPOSE_DOWNLOAD) down --rmi local --volumes --remove-orphans
	$(DOCKER_COMPOSE_TEST) down --rmi local --volumes --remove-orphans
	cargo clean


.PHONY: download
download:
	$(DOCKER_COMPOSE_DOWNLOAD) build
	$(DOCKER_COMPOSE_DOWNLOAD) run --rm download


.PHONY: test
test:
	$(DOCKER_COMPOSE_TEST) build
	$(DOCKER_COMPOSE_TEST) run --rm test


.PHONY: tool
tool:
	cargo install cargo-chef --locked
	cargo install diesel_cli --no-default-features --features sqlite


.PHONY: chef
chef:
	cargo chef prepare --recipe-path recipe.json


.PHONY: _docker_download
_docker_download:
	test -e /app/build/download.sqlite || cp download.sqlite /app/build/download.sqlite
	cargo run --release --bin mocword_download


.PHONY: _docker_test
_docker_test:
	mkdir /app/build
	test -e /app/build/download.sqlite || cp download.sqlite /app/build/download.sqlite
	cargo test --color always
