use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{Shell, cmd};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Xtask {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(short, long)]
        example: String,
        #[arg(short, long, default_value_t = String::from("debug"))]
        profile: String,
    },
}

fn main() {
    let sh = match Shell::new() {
        Ok(sh) => sh,
        Err(err) => return fatal_error(err.into()),
    };
    let xtask = Xtask::parse();
    let result = match &xtask.command {
        Commands::Build { profile, example } => build(sh, example, profile),
    };
    if let Err(err) = result {
        fatal_error(err);
    }
}

fn fatal_error(err: anyhow::Error) {
    eprintln!("{}", err);
    std::process::exit(1);
}

fn build(sh: Shell, example: &str, profile: &str) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    cmd!(
        sh,
        "{cargo} build --profile {profile} --target wasm32-unknown-unknown --example {example}"
    )
    .run()?;
    cmd!(sh, "wasm-bindgen --out-dir web/pkg --out-name {example} --target web target/wasm32-unknown-unknown/{profile}/examples/{example}.wasm").run()?;
    cmd!(
        sh,
        "wasm-opt -Oz --output web/pkg/{example}_bg.wasm.optimized web/pkg/{example}_bg.wasm"
    )
    .run()?;
    sh.copy_file(
        "web/pkg/{example}_bg.wasm.optimized",
        "web/pkg/{example}_bg.wasm",
    )?;
    sh.remove_path("web/pkg/{example}_bg.wasm.optimized")?;
    //XXX figure out index.html wasm path
    Ok(())
}
