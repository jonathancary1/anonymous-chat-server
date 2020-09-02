FROM rust:1.46

WORKDIR /usr/src/anonymous-chat-server
COPY . .

RUN cargo install --path .

CMD ["anonymous-chat-server"]
