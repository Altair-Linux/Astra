use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

/// Astra Package Manager — Modern package manager for Altair Linux
#[derive(Parser)]
#[command(name = "astra", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Astra data directory
    #[arg(long, global = true, default_value = "/var/lib/astra")]
    data_dir: PathBuf,

    /// Root filesystem directory
    #[arg(long, global = true, default_value = "/")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Astra system
    Init,

    /// Manage repositories
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },

    /// Update package indices from repositories
    Update,

    /// Search for packages
    Search {
        /// Search query
        query: String,
    },

    /// Show detailed package information
    Info {
        /// Package name
        package: String,
    },

    /// Install packages
    Install {
        /// Package names to install
        packages: Vec<String>,

        /// Install from a local .astpkg file
        #[arg(long)]
        local: bool,
    },

    /// Remove packages
    Remove {
        /// Package names to remove
        packages: Vec<String>,
    },

    /// Upgrade all packages
    Upgrade,

    /// List installed packages
    List,

    /// Verify installed package integrity
    Verify {
        /// Package name to verify
        package: String,
    },

    /// Build a package from a directory
    Build {
        /// Directory containing Astrafile.yaml
        directory: PathBuf,

        /// Output directory for built package
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Serve a repository directory over HTTP
    ServeRepo {
        /// Repository directory
        directory: PathBuf,

        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        bind: String,
    },

    /// Manage cryptographic keys
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    /// Add a new repository
    Add {
        /// Repository name
        name: String,
        /// Repository URL
        url: String,
    },
    /// Remove a repository
    Remove {
        /// Repository name
        name: String,
    },
    /// List configured repositories
    List,
}

#[derive(Subcommand)]
enum KeyAction {
    /// Generate a new signing key pair
    Generate,
    /// Import a public key
    Import {
        /// Key name
        name: String,
        /// Path to public key file
        path: PathBuf,
    },
    /// Export the public key
    Export {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List trusted keys
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let Cli { json, verbose, data_dir, root, command } = cli;

    // Setup tracing
    let filter = if verbose {
        "astra=debug,astra_core=debug,astra_repo=debug,astra_builder=debug"
    } else {
        "astra=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Reconstruct a Cli without the command for passing to handlers
    let cli = Cli { json, verbose, data_dir, root, command: Commands::Init };

    match command {
        Commands::Init => commands::init(&cli).await,
        Commands::Repo { action } => match action {
            RepoAction::Add { name, url } => commands::repo_add(&cli, &name, &url).await,
            RepoAction::Remove { name } => commands::repo_remove(&cli, &name).await,
            RepoAction::List => commands::repo_list(&cli).await,
        },
        Commands::Update => commands::update(&cli).await,
        Commands::Search { query } => commands::search(&cli, &query).await,
        Commands::Info { package } => commands::info(&cli, &package).await,
        Commands::Install { packages, local } => {
            commands::install(&cli, &packages, local).await
        }
        Commands::Remove { packages } => commands::remove(&cli, &packages).await,
        Commands::Upgrade => commands::upgrade(&cli).await,
        Commands::List => commands::list(&cli).await,
        Commands::Verify { package } => commands::verify(&cli, &package).await,
        Commands::Build { directory, output } => {
            commands::build(&cli, &directory, &output).await
        }
        Commands::ServeRepo { directory, bind } => {
            commands::serve_repo(&cli, &directory, &bind).await
        }
        Commands::Key { action } => match action {
            KeyAction::Generate => commands::key_generate(&cli).await,
            KeyAction::Import { name, path } => commands::key_import(&cli, &name, &path).await,
            KeyAction::Export { output } => commands::key_export(&cli, output.as_deref()).await,
            KeyAction::List => commands::key_list(&cli).await,
        },
    }
}
