use clap::{Parser, Subcommand, ValueEnum};

use yoku_core::parser::llm::{LlmInterface, Ollama};

use anyhow::Result;
use dotenvy::dotenv;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    to_parse: String,

    #[arg(short, long)]
    parser: String
}



#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = Args::parse();
    println!("Parsing with {}", args.parser);
    println!("Parsing: {}", args.to_parse);

    let result = Ollama::parse_set_string(args.to_parse.as_str()).await?;

    println!("{:?}", result);

    Ok(())
}
