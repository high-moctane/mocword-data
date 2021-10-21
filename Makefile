DOCKER_COMPOSE_DOWNLOAD := docker-compose \
		-f docker-compose.download.yml \
		-p mocword_download


.PHONY: build
build:


.PHONY: build-download
build-download:
	$(DOCKER_COMPOSE_DOWNLOAD) build


.PHONY: clean
clean:
	$(DOCKER_COMPOSE_DOWNLOAD) down
	cargo clean
	$(RM) -rf build/*.sqlite


.PHONY: download
download:
	$(DOCKER_COMPOSE_DOWNLOAD) up --abort-on-container-exit --build --remove-orphans


.PHONY: _docker_download
_docker_download:
	test -e /app/build/download.sqlite || cp download.sqlite /app/build/download.sqlite
	cargo run --release --bin mocword_download


.PHONY: tool
tool:
	cargo install cargo-chef --locked
	cargo install diesel_cli --no-default-features --features sqlite


.PHONY: chef
chef:
	cargo chef prepare --recipe-path recipe.json
