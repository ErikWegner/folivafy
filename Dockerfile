## Build stage
## Build mimalloc
FROM alpine:3.18 as mimallocbuilder
RUN apk add git build-base cmake linux-headers
RUN cd /; git clone --depth 1 https://github.com/microsoft/mimalloc; cd mimalloc; mkdir build; cd build; cmake ..; make -j$(nproc); make install

## Build folivafy binary
FROM rust:1.74.1-alpine3.18 AS builder

WORKDIR /usr/src
RUN USER=root cargo new folivafy && cd folivafy && cargo new entity --lib && cargo new migration --lib
COPY Cargo.toml Cargo.lock /usr/src/folivafy/
COPY entity/Cargo.toml /usr/src/folivafy/entity/
COPY migration/Cargo.toml /usr/src/folivafy/migration/
COPY generated /usr/src/folivafy/generated/
WORKDIR /usr/src/folivafy/
RUN apk add --no-cache musl-dev && rustup target add x86_64-unknown-linux-musl
RUN update-ca-certificates
RUN cargo build --target x86_64-unknown-linux-musl --release
COPY src /usr/src/folivafy/src/
COPY entity /usr/src/folivafy/entity/
COPY migration /usr/src/folivafy/migration
RUN touch /usr/src/folivafy/src/main.rs && touch /usr/src/folivafy/entity/src/lib.rs && touch /usr/src/folivafy/migration/src/lib.rs
RUN cargo build --target x86_64-unknown-linux-musl --release
RUN strip -s /usr/src/folivafy/target/x86_64-unknown-linux-musl/release/folivafy

## Put together final image
FROM alpine:3.18 AS runtime
COPY --from=mimallocbuilder /mimalloc/build/*.so.* /lib/
RUN ln -s /lib/libmimalloc.so.2.1 /lib/libmimalloc.so
ENV LD_PRELOAD=/lib/libmimalloc.so
ENV MIMALLOC_LARGE_OS_PAGES=1
COPY --from=builder /usr/src/folivafy/target/x86_64-unknown-linux-musl/release/folivafy .
EXPOSE 3000
USER 65534
CMD ["./folivafy"]

