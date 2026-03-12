use crate::Cli;
use anyhow::Result;
use astra_core::{AstraConfig, PackageManager};
use astra_crypto::PublicKey;
use colored::Colorize;
use std::path::Path;

fn make_config(cli: &Cli) -> AstraConfig {
    AstraConfig {
        root: cli.root.clone(),
        data_dir: cli.data_dir.clone(),
        cache_dir: cli.data_dir.join("cache"),
        repositories: Vec::new(),
    }
}

fn open_manager(cli: &Cli) -> Result<PackageManager> {
    let config_path = cli.data_dir.join("config.json");
    let config = if config_path.exists() {
        AstraConfig::load(&config_path)?
    } else {
        make_config(cli)
    };
    Ok(PackageManager::open(config)?)
}

// ─── init ──────────────────────────────────────────────────────────

pub async fn init(cli: &Cli) -> Result<()> {
    let config = make_config(cli);
    PackageManager::init(config)?;
    if cli.json {
        println!(
            r#"{{"status":"initialized","data_dir":"{}"}}"#,
            cli.data_dir.display()
        );
    } else {
        println!(
            "{} Astra initialized at {}",
            "✓".green().bold(),
            cli.data_dir.display()
        );
    }
    Ok(())
}

// ─── repository ────────────────────────────────────────────────────

pub async fn repo_add(cli: &Cli, name: &str, url: &str) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    mgr.add_repo(name, url)?;
    if cli.json {
        println!(r#"{{"status":"added","name":"{name}","url":"{url}"}}"#);
    } else {
        println!(
            "{} Repository '{}' added: {}",
            "✓".green().bold(),
            name,
            url
        );
    }
    Ok(())
}

pub async fn repo_remove(cli: &Cli, name: &str) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    mgr.remove_repo(name)?;
    if cli.json {
        println!(r#"{{"status":"removed","name":"{name}"}}"#);
    } else {
        println!("{} Repository '{}' removed", "✓".green().bold(), name);
    }
    Ok(())
}

pub async fn repo_list(cli: &Cli) -> Result<()> {
    let mgr = open_manager(cli)?;
    let repos = &mgr.config().repositories;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(repos)?);
    } else if repos.is_empty() {
        println!("No repositories configured.");
    } else {
        println!("{}", "Configured repositories:".bold());
        for repo in repos {
            let status = if repo.enabled {
                "enabled".green()
            } else {
                "disabled".red()
            };
            println!("  {} {} [{}]", repo.name.cyan(), repo.url, status);
        }
    }
    Ok(())
}

// ─── update ────────────────────────────────────────────────────────

pub async fn update(cli: &Cli) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    let updated = mgr.update().await?;
    if cli.json {
        println!(
            r#"{{"status":"updated","repositories":{}}}"#,
            serde_json::to_string(&updated)?
        );
    } else if updated.is_empty() {
        println!("No repositories to update.");
    } else {
        for name in &updated {
            println!("{} Updated repository '{}'", "✓".green().bold(), name);
        }
    }
    Ok(())
}

// ─── search ────────────────────────────────────────────────────────

pub async fn search(cli: &Cli, query: &str) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    mgr.load_cached_indices()?;
    let results = mgr.search(query);
    if cli.json {
        let entries: Vec<_> = results
            .iter()
            .map(|(repo, e)| {
                serde_json::json!({
                    "repo": repo,
                    "name": e.name,
                    "version": e.version.to_string(),
                    "description": e.description,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if results.is_empty() {
        println!("No packages found matching '{}'.", query);
    } else {
        for (repo, entry) in &results {
            let installed = mgr.db().is_installed(&entry.name).unwrap_or(false);
            let marker = if installed {
                " [installed]".green()
            } else {
                "".into()
            };
            println!(
                "{}/{} {} — {}{}",
                repo.dimmed(),
                entry.name.cyan().bold(),
                entry.version.to_string().yellow(),
                entry.description,
                marker,
            );
        }
    }
    Ok(())
}

// ─── info ──────────────────────────────────────────────────────────

pub async fn info(cli: &Cli, name: &str) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    mgr.load_cached_indices()?;

    // check installed first
    let installed = mgr.db().get_package(name).ok();

    // check repos
    let repo_info = mgr.info(name);

    if cli.json {
        let mut obj = serde_json::Map::new();
        if let Some(ref pkg) = installed {
            obj.insert(
                "installed".into(),
                serde_json::json!({
                    "name": pkg.name,
                    "version": pkg.version.to_string(),
                    "architecture": pkg.architecture,
                    "description": pkg.description,
                    "install_date": pkg.install_date.to_rfc3339(),
                    "installed_size": pkg.installed_size,
                    "files": pkg.files.len(),
                }),
            );
        }
        if let Some((repo, entry)) = &repo_info {
            obj.insert(
                "available".into(),
                serde_json::json!({
                    "repo": repo,
                    "name": entry.name,
                    "version": entry.version.to_string(),
                    "description": entry.description,
                }),
            );
        }
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        if let Some(pkg) = &installed {
            println!("{}", "Installed:".bold().green());
            println!("  Name:         {}", pkg.name.cyan());
            println!("  Version:      {}", pkg.version.to_string().yellow());
            println!("  Architecture: {}", pkg.architecture);
            println!("  Description:  {}", pkg.description);
            println!("  Maintainer:   {}", pkg.maintainer);
            println!("  License:      {}", pkg.license);
            println!(
                "  Installed:    {}",
                pkg.install_date.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("  Size:         {} bytes", pkg.installed_size);
            println!("  Files:        {}", pkg.files.len());
        }
        if let Some((repo, entry)) = &repo_info {
            if installed.is_some() {
                println!();
            }
            println!("{}", "Available:".bold().blue());
            println!("  Repository:   {}", repo);
            println!("  Name:         {}", entry.name.cyan());
            println!("  Version:      {}", entry.version.to_string().yellow());
            println!("  Architecture: {}", entry.architecture);
            println!("  Description:  {}", entry.description);
            println!("  License:      {}", entry.license);
        }
        if installed.is_none() && repo_info.is_none() {
            println!("Package '{}' not found.", name);
        }
    }
    Ok(())
}

// ─── install ───────────────────────────────────────────────────────

pub async fn install(cli: &Cli, packages: &[String], local: bool) -> Result<()> {
    let mut mgr = open_manager(cli)?;

    if local {
        for path_str in packages {
            let path = Path::new(path_str);
            let name = mgr.install_local(path, false)?;
            if cli.json {
                println!(r#"{{"status":"installed","package":"{name}"}}"#);
            } else {
                println!(
                    "{} Installed '{}' from local file",
                    "✓".green().bold(),
                    name
                );
            }
        }
    } else {
        let installed = mgr.install(packages).await?;
        if cli.json {
            println!(
                r#"{{"status":"installed","packages":{}}}"#,
                serde_json::to_string(&installed)?
            );
        } else {
            for name in &installed {
                println!("{} Installed '{}'", "✓".green().bold(), name);
            }
            println!(
                "\n{} {} package(s) installed.",
                "✓".green().bold(),
                installed.len()
            );
        }
    }
    Ok(())
}

// ─── remove ────────────────────────────────────────────────────────

pub async fn remove(cli: &Cli, packages: &[String]) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    for name in packages {
        let files = mgr.remove(name)?;
        if cli.json {
            println!(
                r#"{{"status":"removed","package":"{name}","files_removed":{}}}"#,
                files.len()
            );
        } else {
            println!(
                "{} Removed '{}' ({} files)",
                "✓".green().bold(),
                name,
                files.len()
            );
        }
    }
    Ok(())
}

// ─── upgrade ───────────────────────────────────────────────────────

pub async fn upgrade(cli: &Cli) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    mgr.load_cached_indices()?;

    let upgrades = mgr.check_upgrades()?;
    if upgrades.is_empty() {
        if cli.json {
            println!(r#"{{"status":"up-to-date"}}"#);
        } else {
            println!("{} All packages are up to date.", "✓".green().bold());
        }
        return Ok(());
    }

    if !cli.json {
        println!("{}", "Available upgrades:".bold());
        for (name, from, to) in &upgrades {
            println!(
                "  {} {} → {}",
                name.cyan(),
                from.to_string().red(),
                to.to_string().green()
            );
        }
        println!();
    }

    let upgraded = mgr.upgrade().await?;
    if cli.json {
        println!(
            r#"{{"status":"upgraded","packages":{}}}"#,
            serde_json::to_string(&upgraded)?
        );
    } else {
        println!(
            "{} {} package(s) upgraded.",
            "✓".green().bold(),
            upgraded.len()
        );
    }
    Ok(())
}

// ─── list ──────────────────────────────────────────────────────────

pub async fn list(cli: &Cli) -> Result<()> {
    let mgr = open_manager(cli)?;
    let packages = mgr.db().list_packages()?;
    if cli.json {
        let entries: Vec<_> = packages
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "version": p.version.to_string(),
                    "architecture": p.architecture,
                    "description": p.description,
                    "install_date": p.install_date.to_rfc3339(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if packages.is_empty() {
        println!("No packages installed.");
    } else {
        println!(
            "{} {} package(s) installed:\n",
            "●".blue().bold(),
            packages.len()
        );
        for pkg in &packages {
            println!(
                "  {} {} — {}",
                pkg.name.cyan().bold(),
                pkg.version.to_string().yellow(),
                pkg.description,
            );
        }
    }
    Ok(())
}

// ─── verify ────────────────────────────────────────────────────────

pub async fn verify(cli: &Cli, name: &str) -> Result<()> {
    let mgr = open_manager(cli)?;
    let issues = mgr.verify_installed(name)?;
    if cli.json {
        println!(
            r#"{{"package":"{}","verified":{},"issues":{}}}"#,
            name,
            issues.is_empty(),
            serde_json::to_string(&issues)?
        );
    } else if issues.is_empty() {
        println!(
            "{} Package '{}' verified successfully — all files intact.",
            "✓".green().bold(),
            name
        );
    } else {
        println!(
            "{} Package '{}' has {} issue(s):",
            "✗".red().bold(),
            name,
            issues.len()
        );
        for issue in &issues {
            println!("  {} {}", "•".red(), issue);
        }
    }
    Ok(())
}

// ─── build ─────────────────────────────────────────────────────────

pub async fn build(cli: &Cli, directory: &Path, output: &Path) -> Result<()> {
    let mgr = open_manager(cli)?;
    let pkg_path = mgr.build(directory, output)?;
    if cli.json {
        println!(r#"{{"status":"built","path":"{}"}}"#, pkg_path.display());
    } else {
        println!(
            "{} Package built: {}",
            "✓".green().bold(),
            pkg_path.display()
        );
    }
    Ok(())
}

// ─── serve repository ──────────────────────────────────────────────

pub async fn serve_repo(_cli: &Cli, directory: &Path, bind: &str) -> Result<()> {
    let addr: std::net::SocketAddr = bind.parse()?;
    println!(
        "{} Serving repository from {} on http://{}",
        "●".blue().bold(),
        directory.display(),
        addr
    );
    astra_repo_server::serve_repository(directory, addr).await?;
    Ok(())
}

// ─── key management ────────────────────────────────────────────────

pub async fn key_generate(cli: &Cli) -> Result<()> {
    let mgr = open_manager(cli)?;
    let keypair = mgr.generate_keypair()?;
    let pubkey = keypair.public_key();
    if cli.json {
        println!(
            r#"{{"status":"generated","public_key":"{}"}}"#,
            pubkey.to_base64()
        );
    } else {
        println!("{} Signing key generated.", "✓".green().bold());
        println!("  Public key: {}", pubkey.to_base64());
        println!(
            "  Key saved to: {}",
            mgr.config().signing_key_path().display()
        );
    }
    Ok(())
}

pub async fn key_import(cli: &Cli, name: &str, path: &Path) -> Result<()> {
    let mut mgr = open_manager(cli)?;
    let pubkey = PublicKey::load_from_file(path)?;
    mgr.import_key(name, pubkey)?;
    if cli.json {
        println!(r#"{{"status":"imported","name":"{name}"}}"#);
    } else {
        println!(
            "{} Public key '{}' imported from {}",
            "✓".green().bold(),
            name,
            path.display()
        );
    }
    Ok(())
}

pub async fn key_export(cli: &Cli, output: Option<&Path>) -> Result<()> {
    let mgr = open_manager(cli)?;
    let pubkey = mgr.export_public_key()?;
    let b64 = pubkey.to_base64();
    if let Some(path) = output {
        pubkey.save_to_file(path)?;
        if cli.json {
            println!(r#"{{"status":"exported","path":"{}"}}"#, path.display());
        } else {
            println!(
                "{} Public key exported to {}",
                "✓".green().bold(),
                path.display()
            );
        }
    } else if cli.json {
        println!(r#"{{"public_key":"{}"}}"#, b64);
    } else {
        println!("{}", b64);
    }
    Ok(())
}

pub async fn key_list(cli: &Cli) -> Result<()> {
    let mgr = open_manager(cli)?;
    let keyring = mgr.keyring();
    let keys = keyring.all_keys();
    if cli.json {
        let entries: Vec<_> = keys
            .iter()
            .map(|(name, key)| serde_json::json!({"name": name, "key": key.to_base64()}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if keys.is_empty() {
        println!("No trusted keys in keyring.");
    } else {
        println!("{}", "Trusted keys:".bold());
        for (name, key) in keys {
            println!("  {} {}", name.cyan(), key.to_base64().dimmed());
        }
    }
    Ok(())
}
