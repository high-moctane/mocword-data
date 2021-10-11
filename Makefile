DOCKER_COMPOSE_DOWNLOAD := docker-compose \
		-f docker-compose.download.yml \
		-p mocword_download

.PHONY: run
run:


.PHONY: clean
clean:
	$(DOCKER_COMPOSE_DOWNLOAD) down
	cargo clean
	$(RM) -rf build/*.sqlite

.PHONY: download
download:
	$(DOCKER_COMPOSE_DOWNLOAD) up --abort-on-container-exit --build --remove-orphans
