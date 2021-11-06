FROM rust:1.56 AS planner
WORKDIR app
RUN set -xe \
    && cargo install cargo-chef --locked \
    && true
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM rust:1.56 AS builder
WORKDIR app
COPY --from=planner /usr/local/cargo /usr/local/cargo
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release


FROM debian:bullseye-slim AS runtime
WORKDIR app
RUN set -xe \
    && apt-get update \
    && apt-get install -y default-mysql-client \
    && rm -rf /var/lib/apt/lists/* \
    && true
COPY --from=builder /app/target/release/mocword-data /usr/local/bin
ENTRYPOINT ["/usr/local/bin/mocword-data"]
