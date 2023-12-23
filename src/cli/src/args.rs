use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[arg(long)]
    pub host: String,
    #[arg(long)]
    pub key: String,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Packages {
        #[command(subcommand)]
        command: PackageCommands
    },
    Logs {
        package: String,
        log_type: String
    },
}

#[derive(Subcommand, Debug)]
pub enum PackageCommands {
    List {},
    Rebuild { packages: Vec<String> }
}