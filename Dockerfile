FROM rust:1.50

COPY . .

RUN ls

RUN cargo build --release

CMD ["./target/release/crawler"]