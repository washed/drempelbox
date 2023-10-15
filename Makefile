.PHONY: build up install

# build:
# 	cargo build

up:
	@echo "Starting spotify_player daemon..."
	spotify_player -d
	@echo "spotify_player daemon started!"
	cargo run

install:
	cargo install spotify_player --features daemon
	spotify_player authenticate

build:
	-docker buildx create --use --name drempelbox-builder --platform linux/amd64,linux/arm64
	docker buildx build --platform linux/amd64,linux/arm64 -t drempelbox:latest --output docker_build .
	docker buildx build -t drempelbox:latest --load --progress plain .

debug-build:
	-docker buildx create --use --name larger_log --platform linux/arm64 --driver-opt env.BUILDKIT_STEP_LOG_MAX_SIZE=50000000
	docker buildx build --platform linux/arm64 -t drempelbox:latest --load --progress plain --no-cache .
