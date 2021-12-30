.PHONY: build
build:
	cd cmd/mocword_download && go build


.PHONY: download
download:
	cmd/mocword_download/mocword_download


.PHONY: test
test:
	go test ./...
