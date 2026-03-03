use multiws_core::spawn_core;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let core = spawn_core();
    let port = std::env::var("MULTIWS_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3001);
    server::run_server_with_core(core, port).await
}
