FROM metalwarrior665/rust-crawler

COPY . .

RUN cargo build --release

CMD ["./target/release/crawler"]