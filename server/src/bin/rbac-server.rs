#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aio_plugin_rbac_server::hosting::run().await
}
