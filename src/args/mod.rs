use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub sign: bool,

    #[clap(short, long, default_value_t = 8888)]
    pub port: u16,

    #[clap(short, long, default_value_t = String::from("config/config.json"))]
    pub config_path: String,

    #[clap(short = 'L', long, default_value_t = String::from("debug"))]
    pub log_level: String,

    #[clap(short = 'l', long, default_value_t = String::from("aur-build-server.log"))]
    pub log_path: String
}