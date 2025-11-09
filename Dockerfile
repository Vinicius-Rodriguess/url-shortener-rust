FROM rust:1.84

WORKDIR /app

COPY . .

RUN cargo build --release

EXPOSE 3000

CMD ["cargo", "run", "--release"]