# VelaOS top-level convenience targets. See packaging/e2e/README.md.
.PHONY: help e2e glass boot

help:
	@echo "make boot   - build native Microkit and verify persistence across two QEMU boots"
	@echo "make e2e    - QEMU verify-bridge-e2e (Docker + Rust), log in .local/e2e/"
	@echo "make glass  - Glass host demo on http://127.0.0.1:7878/ (M0=path/to/poc/m0 optional)"

e2e:
	bash scripts/verify-bridge-e2e.sh

boot:
	bash run-velaos.sh

M0 ?=
glass:
	cargo run --release --manifest-path poc/product/glass/Cargo.toml -- $(if $(M0),--m0 "$(M0)")
