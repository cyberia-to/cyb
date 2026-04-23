use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mc::Graph;

#[derive(Parser)]
#[command(name = "mc", version, about = "model compilation: .graph → .model")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print a summary of a `.graph` file.
    Inspect { path: PathBuf },

    /// Compile a `.graph` into a `.model` (CT-1 pipeline).
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Inspect { path } => inspect(&path)?,
        Cmd::Compile { input, output } => compile(&input, &output)?,
    }
    Ok(())
}

fn inspect(path: &std::path::Path) -> anyhow::Result<()> {
    let g = Graph::open(path)?;
    println!("name:       {}", g.name());
    println!("sections:   {}", g.frontmatter().files.len());
    for f in &g.frontmatter().files {
        match f.size {
            Some(sz) => println!("  - {} ({}, {} bytes)", f.name, f.format, sz),
            None => println!("  - {} ({}, text)", f.name, f.format),
        }
    }
    let count = g.cyberlinks()?.count();
    println!("cyberlinks: {count}");
    Ok(())
}

fn compile(_input: &std::path::Path, _output: &std::path::Path) -> anyhow::Result<()> {
    anyhow::bail!("compile pipeline not yet implemented (phases 1-8); inspect works")
}
