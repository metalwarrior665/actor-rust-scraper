FROM rust:1.50-slim

COPY . .

RUN ls

RUN cargo build --release

CMD ["./target/release/crawler"]