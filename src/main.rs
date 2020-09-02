mod client;
mod protocol;

#[tokio::main]
async fn main() {
    protocol::listen("0.0.0.0:3000").await;
}
