//! sp — SuperPasswdKey CLI（design_docs/03）。
//!
//! 退出码（SP-F-204）：0 成功 / 1 一般 / 2 凭据 / 3 未找到 / 4 参数。

mod commands;
mod config;
mod f8;
mod integrity;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use sp_core::SpError;

#[derive(Parser)]
#[command(
    name = "sp",
    version,
    about = "Local-first, zero-knowledge secret manager (KDBX4)"
)]
struct Cli {
    /// Database path (overrides config default)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    /// Keyfile path (second factor)
    #[arg(long, global = true)]
    keyfile: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new database (SP-F-101)
    Init {
        /// KDF calibration target in milliseconds (default 700)
        #[arg(long, default_value_t = 700)]
        kdf_target_ms: u64,
    },
    /// Entry operations
    Item {
        #[command(subcommand)]
        op: ItemOp,
    },
    /// Read a single value by URI: <title>/<field> (SP-F-202)
    Read {
        uri: String,
        /// No trailing newline
        #[arg(short = 'n', long)]
        no_newline: bool,
        /// Write to file (ACL restricted to current user)
        #[arg(long)]
        out_file: Option<PathBuf>,
    },
    /// Copy a value to clipboard with timeout clear (SP-F-203)
    Clip {
        uri: String,
        #[arg(long)]
        timeout: Option<u64>,
    },
}

#[derive(Subcommand)]
enum ItemOp {
    /// Create an entry
    Create {
        title: String,
        #[arg(short = 'u', long)]
        username: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Custom field k=v (repeatable)
        #[arg(long, value_parser = parse_kv)]
        field: Vec<(String, String)>,
        /// TOTP otpauth:// URL
        #[arg(long)]
        totp: Option<String>,
        /// Attach a file
        #[arg(long)]
        attach: Option<PathBuf>,
    },
    /// Show an entry
    Get {
        title: String,
        /// Show a single field
        #[arg(short = 'a', long)]
        field: Option<String>,
        /// Show current TOTP code
        #[arg(short = 't', long)]
        totp: bool,
    },
    /// List entry titles
    List,
    /// Delete an entry (recycle bin semantics)
    Delete { title: String },
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("expected k=v, got '{s}'"))
}

fn main() {
    let cli = Cli::parse();
    let code = match run(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("sp: {e}");
            e.exit_code()
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<(), SpError> {
    let db = cli.db.as_deref();
    let keyfile = cli.keyfile.as_deref();
    match cli.command {
        Command::Init { kdf_target_ms } => commands::init(db, keyfile, kdf_target_ms),
        Command::Item { op } => match op {
            ItemOp::Create {
                title,
                username,
                url,
                notes,
                field,
                totp,
                attach,
            } => commands::item_create(commands::CreateOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                title,
                username,
                url,
                notes,
                fields: field,
                totp,
                attach,
            }),
            ItemOp::Get { title, field, totp } => {
                commands::item_get(db, keyfile, title, field, totp)
            }
            ItemOp::List => commands::item_list(db, keyfile),
            ItemOp::Delete { title } => commands::item_delete(db, keyfile, title),
        },
        Command::Read {
            uri,
            no_newline,
            out_file,
        } => commands::read(db, keyfile, uri, no_newline, out_file),
        Command::Clip { uri, timeout } => commands::clip(db, keyfile, uri, timeout),
    }
}
