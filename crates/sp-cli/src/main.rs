//! sp — SuperPasswdKey CLI（design_docs/03）。
//!
//! 退出码（SP-F-204）：0 成功 / 1 一般 / 2 凭据 / 3 未找到 / 4 参数。

use sp_cli::{commands, f8};

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
    /// Attachment operations (SP-F-104)
    Attachment {
        #[command(subcommand)]
        op: AttachmentOp,
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
    /// Run a command with secrets injected as env vars (F8, SP-F-301)
    Run {
        /// .env file with sp:// references
        #[arg(long)]
        env_file: Option<PathBuf>,
        /// Disable output redaction (debug only)
        #[arg(long)]
        no_masking: bool,
        /// Pass secrets via NUL-delimited stdin records instead of env vars
        /// (avoids /proc/<pid>/environ exposure; POSIX, design_docs/04 §4.1)
        #[arg(long)]
        stdin: bool,
        /// Command and args after --
        #[arg(last = true, required = true)]
        cmd: Vec<String>,
    },
    /// Render a template with secrets (F8, SP-F-302)
    Inject {
        /// Template file with {{ sp://title/field }} placeholders
        #[arg(short = 'i', long)]
        input: PathBuf,
        /// Output file (ACL restricted; contains plaintext!)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Stream rendered output to stdout instead of a file
        #[arg(long)]
        stdout: bool,
    },
    /// Copy a value to clipboard with timeout clear (SP-F-203)
    Clip {
        uri: String,
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Bulk import (F9): dotenv | csv | bw-json | env | files
    Import {
        #[arg(long, global = true)]
        dry_run: bool,
        #[command(subcommand)]
        kind: ImportKind,
    },
    /// Export everything (F10): default encrypted archive; --plaintext for JSON
    Export {
        /// Encrypted archive output path (age + zip)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Plaintext JSON to stdout (requires password confirmation)
        #[arg(long)]
        plaintext: bool,
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Subcommand)]
enum ImportKind {
    /// .env file or directory (recursive .env*)
    Dotenv { path: PathBuf },
    /// Generic CSV with column alias mapping
    Csv { path: PathBuf },
    /// Bitwarden JSON export (core fields)
    BwJson { path: PathBuf },
    /// Current process environment variables (in-memory scan)
    Env,
    /// Scan files for secrets (gitleaks rules, recursive)
    Files { path: PathBuf },
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
        /// Tag for categorization (repeatable; KDBX4 native Tags, KeePassXC compatible)
        #[arg(long)]
        tag: Vec<String>,
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
    /// List entry titles (optionally filtered by tag)
    List {
        /// Only entries carrying this tag (exact, case-insensitive)
        #[arg(long)]
        tag: Option<String>,
    },
    /// Delete an entry (recycle bin semantics)
    Delete { title: String },
}

#[derive(Subcommand)]
enum AttachmentOp {
    /// Add or replace an attachment on an existing entry
    Add {
        title: String,
        /// File to attach (any binary)
        file: PathBuf,
        /// Attachment name (defaults to the source file name)
        #[arg(long)]
        name: Option<String>,
    },
    /// List attachment names of an entry
    List { title: String },
    /// Write an attachment out (file with restricted ACL, or raw stdout)
    Get {
        title: String,
        name: String,
        /// Write to file (ACL restricted to current user); without it, raw bytes go to stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
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
                tag,
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
                tags: tag,
                attach,
            }),
            ItemOp::Get { title, field, totp } => {
                commands::item_get(db, keyfile, title, field, totp)
            }
            ItemOp::List { tag } => commands::item_list(db, keyfile, tag.as_deref()),
            ItemOp::Delete { title } => commands::item_delete(db, keyfile, title),
        },
        Command::Attachment { op } => match op {
            AttachmentOp::Add { title, file, name } => {
                commands::attachment_add(db, keyfile, &title, &file, name.as_deref())
            }
            AttachmentOp::List { title } => commands::attachment_list(db, keyfile, &title),
            AttachmentOp::Get { title, name, out } => {
                commands::attachment_get(db, keyfile, &title, &name, out)
            }
        },
        Command::Read {
            uri,
            no_newline,
            out_file,
        } => commands::read(db, keyfile, uri, no_newline, out_file),
        Command::Clip { uri, timeout } => commands::clip(db, keyfile, uri, timeout),
        Command::Run {
            env_file,
            no_masking,
            stdin,
            cmd,
        } => {
            let code = f8::run(db, keyfile, env_file, no_masking, stdin, cmd)?;
            std::process::exit(code);
        }
        Command::Inject {
            input,
            output,
            stdout,
        } => f8::inject(db, keyfile, input, output, stdout),
        Command::Import { kind, dry_run } => match kind {
            ImportKind::Dotenv { path } => commands::import(commands::ImportOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                kind: "dotenv".into(),
                path: Some(path),
                dry_run,
            }),
            ImportKind::Csv { path } => commands::import(commands::ImportOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                kind: "csv".into(),
                path: Some(path),
                dry_run,
            }),
            ImportKind::BwJson { path } => commands::import(commands::ImportOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                kind: "bw-json".into(),
                path: Some(path),
                dry_run,
            }),
            ImportKind::Env => commands::import(commands::ImportOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                kind: "env".into(),
                path: None,
                dry_run,
            }),
            ImportKind::Files { path } => commands::import(commands::ImportOptions {
                cli_db: cli.db.clone(),
                cli_keyfile: cli.keyfile.clone(),
                kind: "files".into(),
                path: Some(path),
                dry_run,
            }),
        },
        Command::Export {
            out,
            plaintext,
            stdout,
        } => commands::export_all(commands::ExportOptions {
            cli_db: cli.db.clone(),
            cli_keyfile: cli.keyfile.clone(),
            out,
            plaintext,
            stdout,
        }),
    }
}
