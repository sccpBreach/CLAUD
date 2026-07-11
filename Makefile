# Pembungkus tipis ke `cargo build`, supaya kompatibel dengan OpenBench
# (yang selalu memanggil `make EXE=<nama>` walau enginenya bukan C/C++).
EXE ?= engine

.PHONY: default
default:
	cargo build --release
	cp target/release/engine $(EXE)
