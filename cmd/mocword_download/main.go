package main

import (
	"context"
	"log"

	"github.com/high-moctane/mocword"
)

func main() {
	if err := mocword.RunDownload(context.Background()); err != nil {
		log.Fatal(err)
	}
}
