mod common;
mod config;
mod db;
mod api;
mod dashboard;
mod master;
mod agent;

use std::error::Error;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(version = "0.2.0", author = "djkcyl")]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 运行为主节点服务器模式
    Master(config::MasterConfig),
    
    /// 运行为从节点客户端模式
    Agent(config::AgentConfig),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Command::Master(config) => {
            println!("启动主节点服务器...");
            master::run(config).await
        }
        Command::Agent(config) => {
            println!("启动从节点客户端...");
            agent::run(config).await
        }
    }
}
