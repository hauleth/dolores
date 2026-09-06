use color_eyre::eyre;
use tracing_subscriber::filter::LevelFilter;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    let app = dolores::cli::App::new();

    tracing_subscriber::fmt::fmt()
        .with_max_level(if app.debug {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        })
        .with_file(true)
        .with_line_number(true)
        .init();

    app.run()
}
