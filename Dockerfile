FROM rustlang/rust

COPY . .

RUN ls

RUN cargo build --release

CMD ["./target/release/crawler"]