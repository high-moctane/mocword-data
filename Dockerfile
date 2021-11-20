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

FROM mariadb:10.6 AS runtime
WORKDIR app
ENV RUST_BACKTRACE=1
RUN set -xe \
    && apt-get update \
    && apt-get install -y sudo \
    && rm -rf /var/lib/apt/lists/* \
    && true
COPY --from=builder /app/target/release/mocword-data /usr/local/bin
COPY entrypoint.sh .
# ENTRYPOINT ["/usr/local/bin/mocword-data"]
ENTRYPOINT ["./entrypoint.sh"]
