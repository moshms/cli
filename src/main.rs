mod admin;
mod assets;
mod backup;
mod compose;
mod doctor;
mod envfile;
mod install;
mod profiles;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "mc-cli",
    version,
    about = "Music Cloud installer and administrator"
)]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Install(InstallArgs),
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
    Migrate,
    Export {
        #[arg(long, default_value = "music-cloud.sql")]
        output: PathBuf,
    },
    Import {
        #[arg(long)]
        file: PathBuf,
    },
    Update {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        ghcr_user: Option<String>,
    },
    Doctor,
}

#[derive(Debug, Args)]
struct InstallArgs {
    #[arg(long)]
    library: Option<PathBuf>,
    #[arg(long)]
    domain: Option<String>,
    #[arg(long)]
    email: Option<String>,
    #[arg(long)]
    profiles: Option<String>,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long)]
    admin_user: Option<String>,
    #[arg(long)]
    admin_password: Option<String>,
    #[arg(long)]
    master_key: Option<String>,
    #[arg(long)]
    tunnel_token: Option<String>,
    #[arg(long)]
    ghcr_user: Option<String>,
    #[arg(long)]
    no_start: bool,
    #[arg(long)]
    no_docker: bool,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum ProfilesCommand {
    List,
    Enable { profile: String },
    Disable { profile: String },
}

#[derive(Debug, Subcommand)]
enum AdminCommand {
    Create {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
        #[arg(long, default_value = "admin")]
        role: String,
    },
    ResetPassword {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    List,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Install(args) => install::run(
            &cli.dir,
            install::InstallOptions {
                library: args.library,
                domain: args.domain,
                email: args.email,
                profiles: args.profiles,
                tag: args.tag,
                admin_user: args.admin_user,
                admin_password: args.admin_password,
                master_key: args.master_key,
                tunnel_token: args.tunnel_token,
                ghcr_user: args.ghcr_user,
                no_start: args.no_start,
                no_docker: args.no_docker,
                force: args.force,
            },
        ),
        Command::Profiles { command } => match command {
            ProfilesCommand::List => profiles::list(&cli.dir),
            ProfilesCommand::Enable { profile } => profiles::enable(&cli.dir, &profile),
            ProfilesCommand::Disable { profile } => profiles::disable(&cli.dir, &profile),
        },
        Command::Admin { command } => match command {
            AdminCommand::Create {
                username,
                password,
                role,
            } => admin::create(&cli.dir, &username, &password, &role),
            AdminCommand::ResetPassword { username, password } => {
                admin::reset_password(&cli.dir, &username, &password)
            }
            AdminCommand::List => admin::list(&cli.dir),
        },
        Command::Migrate => {
            compose::run(compose::command(&cli.dir).args(["run", "--rm", "mc-api", "migrate"]))
        }
        Command::Update { tag, ghcr_user } => {
            install::update(&cli.dir, tag.as_deref(), ghcr_user.as_deref())
        }
        Command::Export { output } => backup::export(&cli.dir, &output),
        Command::Import { file } => backup::import(&cli.dir, &file),
        Command::Doctor => doctor::run(&cli.dir),
    }
}
