.PHONY: all cm7 cm4 clean check \
        flash-cm7 flash-cm4 \
        release-cm7 release-cm4

all: cm7 cm4


cm7:
	cargo build --manifest-path cm7/Cargo.toml

cm4:
	cargo build --manifest-path cm4/Cargo.toml


flash-cm7: cm7
	cargo run --manifest-path cm7/Cargo.toml
# NOTE: For CRC validation, build cm4 before cm7:
#   make release-cm4  # (needed by cm7/build.rs)
#   make release-cm7
flash-cm4: cm4
	probe-rs download --chip STM32H755ZITx --verify \
	  cm4/target/thumbv7em-none-eabihf/debug/cm4

clean:
	cargo clean --manifest-path cm7/Cargo.toml
	cargo clean --manifest-path cm4/Cargo.toml

check:
	cargo clippy --manifest-path cm7/Cargo.toml
	cargo clippy --manifest-path cm4/Cargo.toml
	cargo clippy --manifest-path shared/Cargo.toml

release-cm7:
	cargo build --manifest-path cm7/Cargo.toml --release

release-cm4:
	cargo build --manifest-path cm4/Cargo.toml --release
