FROM rust:1.78 AS build
WORKDIR /app
COPY . .
RUN cargo build --release
CMD ["./target/release/app"]
