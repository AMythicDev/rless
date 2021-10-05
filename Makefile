all:
	cargo run
man:
	cargo run --bin man --features build_deps
rustdocs:
	cargo doc --no-deps
