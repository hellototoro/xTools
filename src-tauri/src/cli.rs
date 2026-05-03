use crate::serial::{self, SerialConnectionConfig, SerialManager, SendPayload};
use crate::session::WorkMode;
use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct XToolsHelper {
    commands: Vec<String>,
}

impl XToolsHelper {
    fn new() -> Self {
        Self {
            commands: [
                "help",
                "list",
                "connect",
                "disconnect",
                "send",
                "hex",
                "mode",
                "monitor",
                "terminal",
                "status",
                "clear",
                "exit",
                "quit",
            ]
            .iter()
            .map(|cmd| cmd.to_string())
            .collect(),
        }
    }
}

impl Completer for XToolsHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];
        if input.contains(' ') {
            return Ok((0, Vec::new()));
        }

        let candidates = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: cmd.clone(),
            })
            .collect();

        Ok((0, candidates))
    }
}

impl Hinter for XToolsHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if line.is_empty() || pos < line.len() {
            return None;
        }

        let input = line.trim();
        self.commands
            .iter()
            .find(|cmd| cmd.starts_with(input) && cmd.as_str() != input)
            .map(|cmd| cmd[input.len()..].to_string())
    }
}

impl Highlighter for XToolsHelper {}
impl Validator for XToolsHelper {}
impl Helper for XToolsHelper {}

pub fn run_interactive_repl() {
    print_banner();

    let manager = Arc::new(Mutex::new(SerialManager::new()));
    let connected = Arc::new(AtomicBool::new(false));
    let mode = Arc::new(Mutex::new(WorkMode::Monitor));
    let running = Arc::new(AtomicBool::new(true));

    start_monitor_reader(manager.clone(), connected.clone(), mode.clone(), running.clone());
    install_ctrlc_handler(running.clone());

    let mut rl = Editor::new().expect("无法创建编辑器");
    rl.set_helper(Some(XToolsHelper::new()));

    let history_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("xtools")
        .join("history.txt");
    let _ = rl.load_history(&history_path);

    while running.load(Ordering::SeqCst) {
        match rl.readline("xtools> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);
                match handle_command(input, &manager, &connected, &mode) {
                    CommandResult::Success(message) if !message.is_empty() => println!("{}", message),
                    CommandResult::Success(_) => {}
                    CommandResult::Error(error) => println!("\x1b[31m错误: {}\x1b[0m", error),
                    CommandResult::Exit => {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    CommandResult::EnterTerminal => {
                        run_terminal_loop(&manager, &connected, false);
                        *mode.lock().unwrap() = WorkMode::Monitor;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => println!("^C"),
            Err(ReadlineError::Eof) => break,
            Err(error) => {
                eprintln!("错误: {:?}", error);
                break;
            }
        }
    }

    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(&history_path);
}

enum CommandResult {
    Success(String),
    Error(String),
    Exit,
    EnterTerminal,
}

fn handle_command(
    input: &str,
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    mode: &Arc<Mutex<WorkMode>>,
) -> CommandResult {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let Some((cmd, args)) = parts.split_first() else {
        return CommandResult::Success(String::new());
    };

    match cmd.to_lowercase().as_str() {
        "help" | "h" | "?" => {
            print_help();
            CommandResult::Success(String::new())
        }
        "list" | "ls" => cmd_list_ports(),
        "connect" | "conn" => cmd_connect(args, manager, connected),
        "disconnect" | "disc" => cmd_disconnect(manager, connected),
        "send" | "s" => cmd_send(args, manager, connected, false),
        "hex" => cmd_send(args, manager, connected, true),
        "mode" => cmd_mode(args, mode),
        "monitor" => {
            *mode.lock().unwrap() = WorkMode::Monitor;
            CommandResult::Success("已切换到监视器模式".to_string())
        }
        "terminal" | "term" => {
            if !connected.load(Ordering::SeqCst) {
                return CommandResult::Error("未连接到串口，请先 connect".to_string());
            }
            *mode.lock().unwrap() = WorkMode::Terminal;
            CommandResult::EnterTerminal
        }
        "status" | "st" => cmd_status(connected, mode),
        "clear" | "cls" => {
            print!("\x1b[2J\x1b[1;1H");
            CommandResult::Success(String::new())
        }
        "exit" | "quit" | "q" => CommandResult::Exit,
        _ => CommandResult::Error(format!("未知命令: {}。输入 help 查看帮助", cmd)),
    }
}

fn cmd_list_ports() -> CommandResult {
    match serial::list_available_ports() {
        Ok(ports) if ports.is_empty() => CommandResult::Success("未检测到可用串口".to_string()),
        Ok(ports) => {
            let mut output = String::from("\n可用串口:\n");
            for (index, port) in ports.iter().enumerate() {
                output.push_str(&format!("  [{}] {} - {}\n", index + 1, port.name, port.description));
            }
            CommandResult::Success(output)
        }
        Err(error) => CommandResult::Error(error),
    }
}

fn cmd_connect(
    args: &[&str],
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
) -> CommandResult {
    let Some(port) = args.first() else {
        return CommandResult::Error("用法: connect <串口> [波特率]".to_string());
    };

    let baud_rate = args.get(1).and_then(|baud| baud.parse().ok()).unwrap_or(115200);
    let config = SerialConnectionConfig {
        port: (*port).to_string(),
        baud_rate,
        ..SerialConnectionConfig::default()
    };

    match manager.lock().unwrap().connect_with_config(&config) {
        Ok(()) => {
            connected.store(true, Ordering::SeqCst);
            CommandResult::Success(format!("已连接到 {} @ {} bps，当前为监视器模式", port, baud_rate))
        }
        Err(error) => CommandResult::Error(error),
    }
}

fn cmd_disconnect(manager: &Arc<Mutex<SerialManager>>, connected: &Arc<AtomicBool>) -> CommandResult {
    match manager.lock().unwrap().disconnect() {
        Ok(()) => {
            connected.store(false, Ordering::SeqCst);
            CommandResult::Success("已断开连接".to_string())
        }
        Err(error) => CommandResult::Error(error),
    }
}

fn cmd_send(
    args: &[&str],
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    hex_mode: bool,
) -> CommandResult {
    if !connected.load(Ordering::SeqCst) {
        return CommandResult::Error("未连接到串口".to_string());
    }
    if args.is_empty() {
        return CommandResult::Error(if hex_mode { "用法: hex <十六进制数据>" } else { "用法: send <数据>" }.to_string());
    }

    let data = args.join(" ");
    let payload = SendPayload {
        data: if hex_mode { data.clone() } else { serial::apply_newline(&data, true, "crlf") },
        hex_mode,
    };

    match manager.lock().unwrap().send_payload(&payload) {
        Ok(event) => CommandResult::Success(format!(
            "[{}] TX{}: {}",
            event.timestamp,
            if hex_mode { " HEX" } else { "" },
            data
        )),
        Err(error) => CommandResult::Error(error),
    }
}

fn cmd_mode(args: &[&str], mode: &Arc<Mutex<WorkMode>>) -> CommandResult {
    let Some(next_mode) = args.first() else {
        return CommandResult::Success(format!("当前模式: {:?}", *mode.lock().unwrap()));
    };

    match next_mode.to_lowercase().as_str() {
        "terminal" | "term" => {
            *mode.lock().unwrap() = WorkMode::Terminal;
            CommandResult::Success("已切换到终端模式，输入 terminal 进入交互".to_string())
        }
        "monitor" | "mon" => {
            *mode.lock().unwrap() = WorkMode::Monitor;
            CommandResult::Success("已切换到监视器模式".to_string())
        }
        _ => CommandResult::Error("用法: mode terminal|monitor".to_string()),
    }
}

fn cmd_status(connected: &Arc<AtomicBool>, mode: &Arc<Mutex<WorkMode>>) -> CommandResult {
    let connection = if connected.load(Ordering::SeqCst) { "已连接" } else { "未连接" };
    CommandResult::Success(format!("状态: {}，模式: {:?}", connection, *mode.lock().unwrap()))
}

pub fn run_direct_terminal(port: &str, baud_rate: u32) {
    let manager = Arc::new(Mutex::new(SerialManager::new()));
    let connected = Arc::new(AtomicBool::new(false));
    if connect_direct(&manager, &connected, port, baud_rate).is_err() {
        return;
    }

    println!("已连接到 {} @ {} bps", port, baud_rate);
    println!("提示: 按 Ctrl+C 退出\n");
    run_terminal_loop(&manager, &connected, true);
    let _ = manager.lock().unwrap().disconnect();
}

pub fn run_direct_monitor(port: &str, baud_rate: u32) {
    let manager = Arc::new(Mutex::new(SerialManager::new()));
    let connected = Arc::new(AtomicBool::new(false));
    if connect_direct(&manager, &connected, port, baud_rate).is_err() {
        return;
    }

    let running = Arc::new(AtomicBool::new(true));
    install_ctrlc_handler(running.clone());
    println!("监视 {} @ {} bps，按 Ctrl+C 退出\n", port, baud_rate);

    while running.load(Ordering::SeqCst) && connected.load(Ordering::SeqCst) {
        let entries = manager.lock().unwrap().read_available().unwrap_or_default();
        for entry in entries {
            println!("[{}] RX: {}", entry.timestamp, entry.text.trim_end());
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = manager.lock().unwrap().disconnect();
}

pub fn run_list_ports() {
    match cmd_list_ports() {
        CommandResult::Success(message) => println!("{}", message),
        CommandResult::Error(error) => eprintln!("无法获取串口列表: {}", error),
        _ => {}
    }
}

fn connect_direct(
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    port: &str,
    baud_rate: u32,
) -> Result<(), String> {
    let config = SerialConnectionConfig {
        port: port.to_string(),
        baud_rate,
        ..SerialConnectionConfig::default()
    };

    manager.lock().unwrap().connect_with_config(&config).map(|_| {
        connected.store(true, Ordering::SeqCst);
    }).map_err(|error| {
        eprintln!("连接失败: {}", error);
        error
    })
}

fn run_terminal_loop(
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    exit_on_ctrl_c: bool,
) {
    println!("\x1b[1;32m进入交互式终端模式\x1b[0m");
    println!(
        "\x1b[1;33m提示: 按 {} 退出\x1b[0m\n",
        if exit_on_ctrl_c { "Ctrl+C" } else { "Ctrl+]" }
    );

    if let Err(error) = enable_raw_mode() {
        println!("\x1b[31m无法启用原始模式: {}\x1b[0m", error);
        return;
    }

    let running = Arc::new(AtomicBool::new(true));
    let rx_running = running.clone();
    let rx_manager = manager.clone();
    let rx_connected = connected.clone();
    let rx_handle = thread::spawn(move || {
        while rx_running.load(Ordering::SeqCst) && rx_connected.load(Ordering::SeqCst) {
            let entries = rx_manager.lock().unwrap().read_available().unwrap_or_default();
            for entry in entries {
                print!("{}", entry.text);
                let _ = io::stdout().flush();
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    while running.load(Ordering::SeqCst) {
        if !event::poll(Duration::from_millis(10)).unwrap_or(false) {
            continue;
        }

        let Ok(Event::Key(key_event)) = event::read() else {
            continue;
        };
        if key_event.kind != KeyEventKind::Press {
            continue;
        }

        if exit_on_ctrl_c
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('c')
        {
            running.store(false, Ordering::SeqCst);
            break;
        }

        if !exit_on_ctrl_c
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char(']')
        {
            running.store(false, Ordering::SeqCst);
            break;
        }

        if let Some(data) = key_to_serial_data(key_event.code, key_event.modifiers) {
            let payload = SendPayload { data, hex_mode: false };
            let _ = manager.lock().unwrap().send_payload(&payload);
        }
    }

    let _ = disable_raw_mode();
    let _ = rx_handle.join();
    println!("\n\x1b[33m已退出终端模式\x1b[0m\n");
}

fn key_to_serial_data(code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    let data = match code {
        KeyCode::Enter => "\r".to_string(),
        KeyCode::Backspace => "\x7f".to_string(),
        KeyCode::Tab => "\t".to_string(),
        KeyCode::Esc => "\x1b".to_string(),
        KeyCode::Up => "\x1b[A".to_string(),
        KeyCode::Down => "\x1b[B".to_string(),
        KeyCode::Right => "\x1b[C".to_string(),
        KeyCode::Left => "\x1b[D".to_string(),
        KeyCode::Home => "\x1b[H".to_string(),
        KeyCode::End => "\x1b[F".to_string(),
        KeyCode::Delete => "\x1b[3~".to_string(),
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => ((c as u8 & 0x1f) as char).to_string(),
        KeyCode::Char(c) => c.to_string(),
        _ => return None,
    };
    Some(data)
}

fn start_monitor_reader(
    manager: Arc<Mutex<SerialManager>>,
    connected: Arc<AtomicBool>,
    mode: Arc<Mutex<WorkMode>>,
    running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            let is_monitor = *mode.lock().unwrap() == WorkMode::Monitor;
            if connected.load(Ordering::SeqCst) && is_monitor {
                let entries = manager.lock().unwrap().read_available().unwrap_or_default();
                for entry in entries {
                    println!("\r\x1b[K[{}] RX: {}", entry.timestamp, entry.text.trim_end());
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
}

fn install_ctrlc_handler(running: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
        println!("\n收到中断信号，正在退出...");
    });
}

fn print_banner() {
    println!(
        r#"
xTools CLI - 串口工具

输入 help 查看可用命令。不带参数启动时默认进入 REPL。
"#
    );
}

fn print_help() {
    println!(
        r#"
可用命令:
  list                         列出可用串口
  connect <串口> [波特率]       连接串口，默认进入监视器模式
  monitor                      切换到监视器模式
  terminal                     进入终端交互模式，Ctrl+] 返回 REPL
  mode terminal|monitor        设置当前模式
  send <数据>                  文本发送，自动追加 CRLF
  hex <十六进制>               HEX 发送
  disconnect                   断开连接
  status                       查看状态
  clear                        清屏
  exit                         退出
"#
    );
}

#[allow(dead_code)]
fn now_timestamp() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}
