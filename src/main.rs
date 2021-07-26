use color_eyre::eyre;
use tracing_subscriber::filter::LevelFilter;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let app = dolores::cli::App::new();

    tracing_subscriber::fmt::fmt()
        .with_max_level(if app.debug { LevelFilter::DEBUG } else { LevelFilter::INFO })
        .init();

    app.run()
}
