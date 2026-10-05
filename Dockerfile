FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock index.html ./
COPY src src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=build /app/target/release/tube /tube
CMD ["/tube"]
