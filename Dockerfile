FROM rust:1.35.1-stretch

ADD ./ /pixel

WORKDIR /pixel

RUN cargo clean
RUN cargo install diesel_cli --no-default-features --features postgres
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release

EXPOSE 3200