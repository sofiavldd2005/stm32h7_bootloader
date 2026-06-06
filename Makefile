.PHONY: all cm7 cm4 flash-cm7 flash-cm4

all: cm7 cm4

cm7:
	cd cm7 && cargo build

cm4:
	cd cm4 && cargo build

flash-cm7:
	cd cm7 && cargo run

flash-cm4:
	cd cm4 && cargo build
	probe-rs download --chip STM32H755ZITx cm4/target/thumbv7em-none-eabihf/debug/cm4
