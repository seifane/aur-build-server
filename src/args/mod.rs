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
}