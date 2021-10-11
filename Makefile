.PHONY: download
download:
	docker-compose -f docker-compose-download.yml -p mocword_download up --build --remove-orphans
