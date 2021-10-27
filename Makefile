.PHONY: tool
tool:
	cargo install diesel_cli --no-default-features --features sqlite


.PHONY: migrate
migrate:
	diesel migration run --database-url "build/download.sqlite"


.PHONY: remigrate
remigrate:
	diesel migration redo --database-url "build/download.sqlite"
