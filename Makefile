.PHONY: build
build:
	cd cmd/mocword_download && go build


.PHONY: download
download:
	cmd/mocword_download/mocword_download


.PHONY: tool
tool:
	go install golang.org/x/tools/cmd/godoc@latest
	go install golang.org/x/tools/cmd/goimports@latest
	go install honnef.co/go/tools/cmd/staticcheck@latest


.PHONY: fmt
fmt:
	find . -name \*.go -exec goimports -w {} \;


.PHONY: test
test:
	go test ./...
