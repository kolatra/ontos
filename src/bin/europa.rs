use voyager::web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    web::server::start().await
}
