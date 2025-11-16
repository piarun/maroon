use clap::Parser;
use std::fs;
use std::path::PathBuf;

mod ir_schema;

#[derive(Parser)]
struct Args {
  #[arg(long)]
  r#in: PathBuf,
  #[arg(long)]
  out: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let in_json = fs::read_to_string(&args.r#in)?;
  let parsed: ir_schema::MaroonIRScenarios = serde_json::from_str(&in_json)?;
  let out_json = serde_json::to_string(&parsed)?;
  fs::write(&args.out, out_json)?;
  Ok(())
}
