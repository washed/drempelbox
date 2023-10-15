
FROM --platform=$TARGETPLATFORM rust:1-bookworm as builder
ARG TARGETPLATFORM
RUN echo $TARGETPLATFORM

RUN apt-get clean && apt-get update && apt-get install libasound2-dev libdbus-1-dev libssl-dev pkg-config -y

FROM builder as spotify-player-builder
RUN cargo install --root /usr/src/spotify_player spotify_player --features daemon

FROM builder as drempelbox-builder

# dummy project to cache deps
WORKDIR /usr/src/
RUN cargo new drempelbox
COPY Cargo.toml Cargo.lock /usr/src//drempelbox/
WORKDIR /usr/src/drempelbox/
RUN cargo build --release

# build with actual source
COPY src/ /usr/src/drempelbox/src/
RUN touch /usr/src/drempelbox/src/main.rs
RUN cargo build --release

FROM scratch AS export
COPY --from=drempelbox-builder /usr/src/drempelbox/target/release/drempelbox ./
COPY --from=spotify-player-builder /usr/src/spotify_player/bin/spotify_player ./
