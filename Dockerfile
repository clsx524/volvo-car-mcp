FROM rust:1-slim-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
# TLS uses rustls with bundled webpki roots, so no ca-certificates is required.
COPY --from=build /build/target/release/vc-cli /usr/local/bin/vc-cli
ENV VOLVO_CONFIG_DIR=/data
EXPOSE 8811
ENTRYPOINT ["/usr/local/bin/vc-cli"]
CMD ["mcp"]
