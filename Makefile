.PHONY: all cm7 cm4 flash

all: cm7 cm4

cm7:
	cd cm7 && cargo build

cm4:
	cd cm4 && cargo build

flash-cm7:
	cd cm7 && cargo run
