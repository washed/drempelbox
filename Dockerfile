
FROM --platform=$TARGETPLATFORM rust:1-bookworm as builder
ARG TARGETPLATFORM
RUN echo $TARGETPLATFORM

RUN apt-get clean && apt-get update && apt-get install libasound2-dev pkg-config -y

FROM builder as drempelbox-builder

# dummy project to cache deps
WORKDIR /usr/src/
RUN cargo new drempelbox
COPY Cargo.toml Cargo.lock /usr/src/drempelbox/
WORKDIR /usr/src/drempelbox/
RUN cargo build --release

# build with actual source
COPY src/ /usr/src/drempelbox/src/
RUN touch /usr/src/drempelbox/src/main.rs
RUN cargo build --release

FROM scratch AS export
COPY --from=drempelbox-builder /usr/src/drempelbox/target/release/drempelbox ./
