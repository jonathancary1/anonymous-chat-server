# Anonymous Chat Server

A Rust chat server written using Tokio.

## Frame Structure

All messages are prefixed with a big endian encoded uint16 of the body length.
The body of a message consists of a UTF-8 encoded string of JSON.

## Deployment

```
docker build -t anonymous-chat-server .
docker run -p 0.0.0.0:3000:3000 --name anonymous-chat-server anonymous-chat-server
```
