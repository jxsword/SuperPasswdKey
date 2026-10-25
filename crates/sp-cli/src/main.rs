use clap::Parser;

/// SuperPasswdKey - local-first, zero-knowledge password/secret manager CLI.
#[derive(Parser)]
#[command(name = "sp", version, about)]
struct Cli {
    /// Database path (overrides default in config.toml)
    #[arg(long, global = true)]
    db: Option<std::path::PathBuf>,
    /// Keyfile path (second factor)
    #[arg(long, global = true)]
    keyfile: Option<std::path::PathBuf>,
    /// Suppress non-error output
    #[arg(long, global = true)]
    quiet: bool,
    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,
}

fn main() {
    let _cli = Cli::parse();
    // v0.1 命令族在 H3/H4 分阶段实现（design_docs/03）
    println!("sp {} — scaffold", env!("CARGO_PKG_VERSION"));
}
