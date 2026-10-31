use rigcraft::{serve, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rigcraft=info,tower_http=info".into()),
        )
        .init();
    serve(Config::from_env()).await
}
