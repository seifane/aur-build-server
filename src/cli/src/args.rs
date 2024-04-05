use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Base url of the server. Will take over the profile if specified along with api-key.
    #[arg(long)]
    pub base_url: Option<String>,
    /// Api key of the server. Will take over the profile if specified along with base-url.
    #[arg(long)]
    pub api_key: Option<String>,
    /// Profile name to use.
    #[arg(long, short)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Get the list of current workers
    Workers {},

    /// Packages related commands. list, rebuild.
    Packages {
        #[command(subcommand)]
        command: PackageCommands
    },
    /// <package> Fetch the logs for the given package.
    Logs {
        package: String,
    },
    /// Webhooks related commands. trigger.
    Webhooks {
        #[command(subcommand)]
        command: WebhookCommands
    },
    /// Profile related commands. list, create, delete, set-default.
    Profiles {
        #[command(subcommand)]
        command: ProfileCommands
    }
}


#[derive(Subcommand, Debug)]
pub enum PackageCommands {
    /// List packages
    List {},
    /// package1 package2 [...] Rebuild specified packages, if no specified packages rebuild all.
    Rebuild { packages: Vec<String> }
}

#[derive(Subcommand, Debug)]
pub enum WebhookCommands {
    /// Manually trigger a webhook
    Trigger {
        #[command(subcommand)]
        command: WebhookTriggerCommands
    },
}

#[derive(Subcommand, Debug)]
pub enum WebhookTriggerCommands {
    /// Manually trigger a PackageUpdated webhook
    PackageUpdated {
        package_name: String
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    /// List profiles.
    List {},
    /// Create a new profile.
    Create {},
    /// Delete a profile.
    Delete { name: String },
    /// Set a profile as default.
    SetDefault { name: String }
}