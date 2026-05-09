pub fn init() {
    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .compact()
        .try_init()
    {
        eprintln!("failed to initialize logging: {error}");
    }
}
