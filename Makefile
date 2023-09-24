.PHONY: build up install

build:
	cargo build

up:
	@echo "Starting spotify_player daemon..."
	spotify_player -d
	@echo "spotify_player daemon started!"
	cargo run

install:
	cargo install spotify_player --features daemon
	spotify_player authenticate
