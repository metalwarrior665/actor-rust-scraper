FROM metalwarrior665/rust-crawler

COPY . .

RUN cargo --version && cargo build --release

CMD ["./target/release/crawler"]