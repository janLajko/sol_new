use std::{env, str::FromStr};

use sol_new::engine::Monitor;

use tracing::Level;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let env_filter = EnvFilter::new("sol_new=debug")  
    .add_directive("warn".parse().unwrap());  

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_env_filter(env_filter)
        .with_target(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");

    let monitor = Monitor::new().await?;
    monitor.run().await?;
    Ok(())
}
