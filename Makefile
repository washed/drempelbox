.PHONY: build build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu

build: build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu

build-aarch64-unknown-linux-gnu:
	cross build --release --target=aarch64-unknown-linux-gnu

build-x86_64-unknown-linux-gnu:
	cross build --release --target=x86_64-unknown-linux-gnu

