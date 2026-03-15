use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

/// astra package manager — modern package manager for altair linux
#[derive(Parser)]
#[command(name = "astra", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// output in json format
    #[arg(long, global = true)]
    json: bool,

    /// enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// astra data directory
    #[arg(long, global = true, default_value = "/var/lib/astra")]
    data_dir: PathBuf,

    /// root filesystem directory
    #[arg(long, global = true, default_value = "/")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// initialize a new astra system
    Init,

    /// manage repositories
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },

    /// update package indices from repositories
    Update,

    /// search for packages
    Search {
        /// search query
        query: String,
    },

    /// show detailed package information
    Info {
        /// package name
        package: String,
    },

    /// install packages
    Install {
        /// package names to install
        packages: Vec<String>,

        /// install from a local .astpkg file
        #[arg(long)]
        local: bool,
    },

    /// remove packages
    Remove {
        /// package names to remove
        packages: Vec<String>,
    },

    /// upgrade all packages
    Upgrade,

    /// list installed packages
    List,

    /// verify installed package integrity
    Verify {
        /// package name to verify
        package: String,
    },

    /// build a package from a directory
    Build {
        /// directory containing Astrafile.yaml
        directory: PathBuf,

        /// output directory for built package
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// enable isolated build execution mode
        #[arg(long)]
        sandbox: bool,
    },

    /// serve a repository directory over http
    ServeRepo {
        /// repository directory
        directory: PathBuf,

        /// bind address
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        bind: String,
    },

    /// manage cryptographic keys
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    /// add a new repository
    Add {
        /// repository name
        name: String,
        /// repository url
        url: String,
    },
    /// remove a repository
    Remove {
        /// repository name
        name: String,
    },
    /// list configured repositories
    List,
    /// generate index.json for a repository directory
    Update {
        /// repository root containing packages/
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// generate a new signing key pair
    Generate,
    /// add a trusted public key
    Add {
        /// key name
        name: String,
        /// path to public key file
        path: PathBuf,
    },
    /// import a public key (alias for add)
    Import {
        /// key name
        name: String,
        /// path to public key file
        path: PathBuf,
    },
    /// export the public key
    Export {
        /// output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// list trusted keys
    List,
    /// remove a trusted key
    Remove {
        /// key name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let Cli {
        json,
        verbose,
        data_dir,
        root,
        command,
    } = cli;

    // set up tracing
    let filter = if verbose {
        "astra=debug,astra_core=debug,astra_repo=debug,astra_builder=debug"
    } else {
        "astra=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // reconstruct a cli without the command for passing to handlers
    let cli = Cli {
        json,
        verbose,
        data_dir,
        root,
        command: Commands::Init,
    };

    match command {
        Commands::Init => commands::init(&cli).await,
        Commands::Repo { action } => match action {
            RepoAction::Add { name, url } => commands::repo_add(&cli, &name, &url).await,
            RepoAction::Remove { name } => commands::repo_remove(&cli, &name).await,
            RepoAction::List => commands::repo_list(&cli).await,
            RepoAction::Update { directory } => commands::repo_update(&cli, &directory).await,
        },
        Commands::Update => commands::update(&cli).await,
        Commands::Search { query } => commands::search(&cli, &query).await,
        Commands::Info { package } => commands::info(&cli, &package).await,
        Commands::Install { packages, local } => commands::install(&cli, &packages, local).await,
        Commands::Remove { packages } => commands::remove(&cli, &packages).await,
        Commands::Upgrade => commands::upgrade(&cli).await,
        Commands::List => commands::list(&cli).await,
        Commands::Verify { package } => commands::verify(&cli, &package).await,
        Commands::Build {
            directory,
            output,
            sandbox,
        } => commands::build(&cli, &directory, &output, sandbox).await,
        Commands::ServeRepo { directory, bind } => {
            commands::serve_repo(&cli, &directory, &bind).await
        }
        Commands::Key { action } => match action {
            KeyAction::Generate => commands::key_generate(&cli).await,
            KeyAction::Add { name, path } => commands::key_import(&cli, &name, &path).await,
            KeyAction::Import { name, path } => commands::key_import(&cli, &name, &path).await,
            KeyAction::Export { output } => commands::key_export(&cli, output.as_deref()).await,
            KeyAction::List => commands::key_list(&cli).await,
            KeyAction::Remove { name } => commands::key_remove(&cli, &name).await,
        },
    }
}
