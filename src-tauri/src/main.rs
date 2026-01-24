// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtools")]
#[command(author = "xTools Team")]
#[command(version = "0.1.0")]
#[command(about = "跨平台超级工具集", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 串口终端工具
    Serial {
        /// 串口名称 (如 COM3 或 /dev/ttyUSB0)
        #[arg(short, long)]
        port: Option<String>,

        /// 波特率
        #[arg(short, long, default_value = "115200")]
        baud: u32,

        /// 终端交互模式
        #[arg(short, long)]
        terminal: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Serial { port, baud, terminal }) => {
            xtools_lib::cli::run_serial_cli(port, baud, terminal);
        }
        None => {
            // 无命令时启动 GUI
            xtools_lib::run();
        }
    }
}
