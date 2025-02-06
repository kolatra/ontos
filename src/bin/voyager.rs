use voyager::scanner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    scanner::start().await
}
