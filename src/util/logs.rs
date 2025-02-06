use env_logger::Builder;
use log::LevelFilter;

pub fn init_logs() -> anyhow::Result<()> {
    let filter = match std::env::var("LOG_LEVEL")? {
        s if s.eq_ignore_ascii_case("trace") => LevelFilter::Trace,
        s if s.eq_ignore_ascii_case("debug") => LevelFilter::Debug,
        s if s.eq_ignore_ascii_case("info") => LevelFilter::Info,
        s if s.eq_ignore_ascii_case("warn") => LevelFilter::Warn,
        s if s.eq_ignore_ascii_case("error") => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    Builder::new()
        .filter_module("common", filter)
        .filter_module("europa", filter)
        .filter_module("voyager", filter)
        .filter_module("ganymede", filter)
        .init();

    Ok(())
}
