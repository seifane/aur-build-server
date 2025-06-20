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
    Workers {
        #[command(subcommand)]
        command: WorkerCommands,
    },
    /// Packages related commands. list, get, add, remove, rebuild.
    Packages {
        #[command(subcommand)]
        command: PackageCommands
    },
    /// Patch related commands. list, add, remove.
    Patches {
        #[command(subcommand)]
        command: PatchCommands
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
pub enum WorkerCommands {
    /// List workers
    List {},
    ///  Evict the worker with the given id
    Evict {
        id: usize
    }
}

#[derive(Subcommand, Debug)]
pub enum PackageCommands {
    /// List packages
    List {
        #[clap(long, short, action)]
        compact: bool,
    },

    /// Get detailed package info
    Get {
        name: String,
    },

    /// Add a new package
    Add {
        name: Option<String>,
        run_before: Option<String>
    },

    /// Remove a package
    Remove {
        name: String
    },

    /// package1 package2 [...] Rebuild specified packages, if no specified packages rebuild all.
    Rebuild {
        packages: Vec<String>,
        #[clap(long, short, action)]
        force: bool
    },
}

#[derive(Subcommand, Debug)]
pub enum PatchCommands {
    /// List patches for packages
    List {
        package_name: String,
    },

    /// Add a new patch for a package
    Add {
        package_name: String,
        url: String,
        sha_512: Option<String>,
    },

    /// Remove a patch for a package
    Remove {
        package_name: String,
        id: i32
    }
}

#[derive(Subcommand, Debug)]
pub enum WebhookCommands {
    /// Manually trigger a webhook
    Trigger {},
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