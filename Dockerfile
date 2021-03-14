FROM metalwarrior665/rust-crawler

COPY . .

RUN ls

RUN cargo build --release

CMD ["./target/release/crawler"]