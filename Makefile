.PHONY: build
build:
	cd cmd/mocword_download && go build


.PHONY: download
download:
	cmd/mocword_download/mocword_download


.PHONY: check
check:
	find . -print | grep --regex '.*\.go' | xargs goimports -w -local "github.com/high-moctane/mocword"
	staticcheck ./...


.PHONY: test
test:
	go test ./...
