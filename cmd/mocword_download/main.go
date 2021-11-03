package main

import (
	"context"
	"log"

	"github.com/high-moctane/mocword/download"
)

func main() {
	if err := download.Run(context.Background()); err != nil {
		log.Fatal(err)
	}
}
