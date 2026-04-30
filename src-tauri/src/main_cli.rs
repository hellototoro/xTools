use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "xtools_cli",
    version,
    about = "xTools CLI - 串口终端",
    long_about = "xTools CLI 串口终端。\n\n示例:\n  xtools_cli connect COM3 115200\n  xtools_cli\n\n说明:\n  - 不带参数进入交互式 REPL\n  - connect 直接进入终端模式，按 Ctrl+C 退出\n"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 直接连接串口并进入终端模式
    Connect {
        /// 串口名称 (如 COM3)
        port: String,
        /// 波特率 (默认 115200)
        baud: Option<u32>,
    },
    /// 列出可用串口
    List,
    /// 进入交互式 REPL
    Repl,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Connect { port, baud }) => {
            let baud_rate = baud.unwrap_or(115200);
            xtools_lib::cli::run_direct_terminal(&port, baud_rate);
        }
        Some(Commands::List) => {
            xtools_lib::cli::run_list_ports();
        }
        Some(Commands::Repl) | None => {
            xtools_lib::cli::run_interactive_repl();
        }
    }
}
