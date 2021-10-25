.PHONY: tool
tool:
	cargo install diesel_cli --no-default-features --features sqlite
