fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

check:
	cargo check

build:
	cargo build

serve:
	dx serve --platform web
