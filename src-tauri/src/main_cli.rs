use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "xtools_cli",
    version,
    about = "xTools CLI - 串口工具",
    long_about = "xTools CLI 串口工具。\n\n示例:\n  xtools_cli terminal COM3 115200\n  xtools_cli monitor COM3 115200\n  xtools_cli\n\n说明:\n  - 不带参数进入交互式 REPL\n  - terminal 进入终端交互模式\n  - monitor 进入串口监视器模式\n"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 直接连接串口并进入终端交互模式
    Terminal {
        /// 串口名称 (如 COM3)
        port: String,
        /// 波特率 (默认 115200)
        baud: Option<u32>,
    },
    /// 直接连接串口并进入监视器模式
    Monitor {
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
        Some(Commands::Terminal { port, baud }) => {
            let baud_rate = baud.unwrap_or(115200);
            xtools_lib::cli::run_direct_terminal(&port, baud_rate);
        }
        Some(Commands::Monitor { port, baud }) => {
            let baud_rate = baud.unwrap_or(115200);
            xtools_lib::cli::run_direct_monitor(&port, baud_rate);
        }
        Some(Commands::List) => {
            xtools_lib::cli::run_list_ports();
        }
        Some(Commands::Repl) | None => {
            xtools_lib::cli::run_interactive_repl();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn defaults_to_repl_without_subcommand() {
        let cli = Cli::try_parse_from(["xtools_cli"]).unwrap();
        assert!(matches!(cli.command, None));
    }

    #[test]
    fn parses_terminal_subcommand() {
        let cli = Cli::try_parse_from(["xtools_cli", "terminal", "COM3", "9600"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Terminal { ref port, baud: Some(9600) }) if port == "COM3"
        ));
    }

    #[test]
    fn parses_monitor_subcommand() {
        let cli = Cli::try_parse_from(["xtools_cli", "monitor", "COM4"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Monitor { ref port, baud: None }) if port == "COM4"
        ));
    }

    #[test]
    fn parses_list_subcommand() {
        let cli = Cli::try_parse_from(["xtools_cli", "list"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::List)));
    }
}
