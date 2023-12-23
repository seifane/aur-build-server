use clap::Parser;
use log::LevelFilter;
use clap::builder::TypedValueParser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value_t = String::from("config_server.json"))]
    pub config_path: String,

    #[arg(
        long,
        default_value = "info",
        value_parser = clap::builder::PossibleValuesParser::new(["off", "error", "warn", "info", "debug", "trace"])
        .map(|s| s.parse::<log::LevelFilter>().unwrap()),
    )]
    pub log_level: LevelFilter,

    #[clap(short = 'l', long, default_value_t = String::from("aur-build-server.log"))]
    pub log_path: String
}