#[tokio::main]
async fn main() -> anyhow::Result<()> {
    horarioshub_api::run().await
}
