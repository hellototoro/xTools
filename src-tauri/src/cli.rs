use crate::serial::{self, SerialManager};
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

// ============ 命令补全助手 ============

struct XToolsHelper {
    commands: Vec<String>,
}

impl XToolsHelper {
    fn new() -> Self {
        Self {
            commands: vec![
                "help".to_string(),
                "list".to_string(),
                "ls".to_string(),
                "connect".to_string(),
                "conn".to_string(),
                "disconnect".to_string(),
                "disc".to_string(),
                "send".to_string(),
                "s".to_string(),
                "hex".to_string(),
                "terminal".to_string(),
                "term".to_string(),
                "config".to_string(),
                "cfg".to_string(),
                "clear".to_string(),
                "cls".to_string(),
                "status".to_string(),
                "st".to_string(),
                "exit".to_string(),
                "quit".to_string(),
                "q".to_string(),
            ],
        }
    }
}

impl Completer for XToolsHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut candidates = Vec::new();
        let input = &line[..pos];
        
        // 如果是第一个单词，补全命令
        if !input.contains(' ') {
            for cmd in &self.commands {
                if cmd.starts_with(input) {
                    candidates.push(Pair {
                        display: cmd.clone(),
                        replacement: cmd.clone(),
                    });
                }
            }
        }
        
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
        
        // 简单的命令提示
        for cmd in &self.commands {
            if cmd.starts_with(input) && cmd != input {
                return Some(cmd[input.len()..].to_string());
            }
        }
        
        None
    }
}

impl Highlighter for XToolsHelper {}
impl Validator for XToolsHelper {}
impl Helper for XToolsHelper {}

// ============ 交互式 REPL ============

pub fn run_interactive_repl() {
    print_banner();
    
    let manager = Arc::new(Mutex::new(SerialManager::new()));
    let running = Arc::new(AtomicBool::new(true));
    let connected = Arc::new(AtomicBool::new(false));
    let in_terminal_mode = Arc::new(AtomicBool::new(false));  // 终端模式标志
    
    // 串口接收线程（仅在非终端模式时显示）
    let manager_rx = manager.clone();
    let running_rx = running.clone();
    let connected_rx = connected.clone();
    let in_terminal_rx = in_terminal_mode.clone();
    
    thread::spawn(move || {
        while running_rx.load(Ordering::SeqCst) {
            // 终端模式时不在这里处理数据
            if connected_rx.load(Ordering::SeqCst) && !in_terminal_rx.load(Ordering::SeqCst) {
                let mut mgr = manager_rx.lock().unwrap();
                match mgr.read_available() {
                    Ok(entries) => {
                        for entry in entries {
                            println!("\r\x1b[K[{}] RX: {}", entry.timestamp, entry.data.trim());
                            // 不重新打印提示符，让 rustyline 处理
                        }
                    }
                    Err(_) => {}
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
    
    // 设置 Ctrl+C 处理
    let running_ctrlc = running.clone();
    ctrlc::set_handler(move || {
        running_ctrlc.store(false, Ordering::SeqCst);
        println!("\n收到中断信号，正在退出...");
        std::process::exit(0);
    })
    .expect("设置 Ctrl+C 处理失败");
    
    // 创建 rustyline 编辑器
    let helper = XToolsHelper::new();
    let mut rl = Editor::new().expect("无法创建编辑器");
    rl.set_helper(Some(helper));
    
    // 加载历史
    let history_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("xtools")
        .join("history.txt");
    
    let _ = rl.load_history(&history_path);
    
    // 主 REPL 循环
    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        
        let readline = rl.readline("xtools> ");
        
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                
                // 添加到历史
                rl.add_history_entry(input)
                    .expect("添加历史失败");
                
                let result = handle_command(input, &manager, &connected, &in_terminal_mode);
                
                match result {
                    CommandResult::Exit => {
                        println!("再见！");
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    CommandResult::Success(msg) => {
                        if !msg.is_empty() {
                            println!("{}", msg);
                        }
                    }
                    CommandResult::Error(err) => {
                        println!("\x1b[31m错误: {}\x1b[0m", err);
                    }
                    CommandResult::EnterTerminal => {
                        // 连接成功，自动进入终端模式
                        run_terminal_mode(&manager, &connected, &in_terminal_mode);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("退出");
                break;
            }
            Err(err) => {
                eprintln!("错误: {:?}", err);
                break;
            }
        }
    }
    
    // 保存历史
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(&history_path);
}

enum CommandResult {
    Success(String),
    Error(String),
    Exit,
    EnterTerminal,  // 连接成功后进入终端模式
}

fn handle_command(
    input: &str,
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    in_terminal_mode: &Arc<AtomicBool>,
) -> CommandResult {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return CommandResult::Success(String::new());
    }
    
    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];
    
    match cmd.as_str() {
        "help" | "h" | "?" => {
            print_help();
            CommandResult::Success(String::new())
        }
        
        "list" | "ls" => {
            cmd_list_ports()
        }
        
        "connect" | "conn" => {
            cmd_connect(args, manager, connected)
        }
        
        "disconnect" | "disc" => {
            cmd_disconnect(manager, connected)
        }
        
        "send" | "s" => {
            cmd_send(args, manager, connected)
        }
        
        "hex" => {
            cmd_send_hex(args, manager, connected)
        }
        
        "terminal" | "term" => {
            cmd_terminal(manager, connected, in_terminal_mode)
        }
        
        "config" | "cfg" => {
            cmd_config(args)
        }
        
        "clear" | "cls" => {
            print!("\x1b[2J\x1b[1;1H");
            CommandResult::Success(String::new())
        }
        
        "status" | "st" => {
            cmd_status(connected)
        }
        
        "exit" | "quit" | "q" => {
            CommandResult::Exit
        }
        
        _ => {
            CommandResult::Error(format!("未知命令: {}。输入 'help' 查看帮助", cmd))
        }
    }
}

fn cmd_list_ports() -> CommandResult {
    match serial::list_available_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                CommandResult::Success("未检测到可用串口".to_string())
            } else {
                let mut output = String::from("\n可用串口:\n");
                for (i, port) in ports.iter().enumerate() {
                    output.push_str(&format!("  [{}] {} - {}\n", i + 1, port.name, port.description));
                }
                CommandResult::Success(output)
            }
        }
        Err(e) => CommandResult::Error(e),
    }
}

fn cmd_connect(
    args: &[&str],
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("用法: connect <串口> [波特率]".to_string());
    }
    
    let port = args[0];
    let baud = if args.len() > 1 {
        args[1].parse::<u32>().unwrap_or(115200)
    } else {
        115200
    };
    
    let mut mgr = manager.lock().unwrap();
    match mgr.connect(port, baud, 8, 1, "none") {
        Ok(_) => {
            connected.store(true, Ordering::SeqCst);
            println!("\n✓ 已连接到 {} @ {} bps\n", port, baud);
            CommandResult::EnterTerminal
        }
        Err(e) => CommandResult::Error(e),
    }
}

fn cmd_disconnect(
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
) -> CommandResult {
    let mut mgr = manager.lock().unwrap();
    match mgr.disconnect() {
        Ok(_) => {
            connected.store(false, Ordering::SeqCst);
            CommandResult::Success("✓ 已断开连接".to_string())
        }
        Err(e) => CommandResult::Error(e),
    }
}

fn cmd_send(
    args: &[&str],
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
) -> CommandResult {
    if !connected.load(Ordering::SeqCst) {
        return CommandResult::Error("未连接到串口".to_string());
    }
    
    if args.is_empty() {
        return CommandResult::Error("用法: send <数据>".to_string());
    }
    
    let data = args.join(" ");
    let mut mgr = manager.lock().unwrap();
    
    match mgr.send(&format!("{}\r\n", data), false) {
        Ok(_) => {
            let now = Local::now();
            let timestamp = now.format("%H:%M:%S%.3f").to_string();
            CommandResult::Success(format!("[{}] TX: {}", timestamp, data))
        }
        Err(e) => CommandResult::Error(e),
    }
}

fn cmd_send_hex(
    args: &[&str],
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
) -> CommandResult {
    if !connected.load(Ordering::SeqCst) {
        return CommandResult::Error("未连接到串口".to_string());
    }
    
    if args.is_empty() {
        return CommandResult::Error("用法: hex <十六进制数据>".to_string());
    }
    
    let data = args.join(" ");
    let mut mgr = manager.lock().unwrap();
    
    match mgr.send(&data, true) {
        Ok(_) => {
            let now = Local::now();
            let timestamp = now.format("%H:%M:%S%.3f").to_string();
            CommandResult::Success(format!("[{}] TX HEX: {}", timestamp, data))
        }
        Err(e) => CommandResult::Error(e),
    }
}

// 运行交互式终端模式
fn run_terminal_mode(
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    in_terminal_mode: &Arc<AtomicBool>,
) {
    // 标记进入终端模式，暂停主 REPL 的接收线程
    in_terminal_mode.store(true, Ordering::SeqCst);
    
    println!("\x1b[1;32m═══════════════════════════════════════════\x1b[0m");
    println!("\x1b[1;32m   进入交互式终端模式\x1b[0m");
    println!("\x1b[1;33m   重要: 按 Ctrl+] 退出到命令行模式\x1b[0m");
    println!("\x1b[1;32m═══════════════════════════════════════════\x1b[0m\n");
    
    // 使用 crossterm 启用原始模式（跨平台）
    if let Err(e) = enable_raw_mode() {
        println!("\x1b[31m无法启用原始模式: {}\x1b[0m", e);
        in_terminal_mode.store(false, Ordering::SeqCst);
        return;
    }
    
    let running = Arc::new(AtomicBool::new(true));
    let running_rx = running.clone();
    let manager_rx = manager.clone();
    let connected_rx = connected.clone();
    
    // 接收线程 - 显示串口数据
    let rx_handle = thread::spawn(move || {
        while running_rx.load(Ordering::SeqCst) && connected_rx.load(Ordering::SeqCst) {
            let mut mgr = manager_rx.lock().unwrap();
            match mgr.read_available() {
                Ok(entries) => {
                    for entry in entries {
                        // 直接输出数据，不添加时间戳
                        print!("{}", entry.data);
                        let _ = io::stdout().flush();
                    }
                }
                Err(_) => {}
            }
            drop(mgr);
            thread::sleep(Duration::from_millis(10));
        }
    });
    
    // 主循环 - 读取键盘输入并发送 (使用 crossterm 跨平台)
    loop {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                // 只处理按下事件，忽略释放和重复事件
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }
                
                // Ctrl+] 退出
                if key_event.modifiers.contains(KeyModifiers::CONTROL) 
                    && key_event.code == KeyCode::Char(']') 
                {
                    running.store(false, Ordering::SeqCst);
                    break;
                }
                
                let data = match key_event.code {
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
                    KeyCode::Char(c) => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            // Ctrl+字母 转换为控制字符
                            let ctrl_char = (c as u8 & 0x1f) as char;
                            ctrl_char.to_string()
                        } else {
                            c.to_string()
                        }
                    }
                    _ => continue,
                };
                
                let mut mgr = manager.lock().unwrap();
                let _ = mgr.send(&data, false);
            }
        }
    }
    
    let _ = disable_raw_mode();
    let _ = rx_handle.join();
    
    // 退出终端模式
    in_terminal_mode.store(false, Ordering::SeqCst);
    
    println!("\n\x1b[33m═══ 已退出终端模式 ═══\x1b[0m\n");
}

// 交互式终端模式命令
fn cmd_terminal(
    manager: &Arc<Mutex<SerialManager>>,
    connected: &Arc<AtomicBool>,
    in_terminal_mode: &Arc<AtomicBool>,
) -> CommandResult {
    if !connected.load(Ordering::SeqCst) {
        return CommandResult::Error("未连接到串口，请先使用 connect 命令连接".to_string());
    }
    
    run_terminal_mode(manager, connected, in_terminal_mode);
    CommandResult::Success(String::new())
}

fn cmd_config(args: &[&str]) -> CommandResult {
    if args.is_empty() {
        let output = "
配置选项:
  baud <速率>     - 设置波特率 (默认: 115200)
  data <位数>     - 设置数据位 (5-8)
  stop <位数>     - 设置停止位 (1-2)
  parity <类型>   - 设置校验 (none/odd/even)

示例: config baud 9600
";
        return CommandResult::Success(output.to_string());
    }
    
    // TODO: 实现配置功能
    CommandResult::Success("配置已更新（功能待实现）".to_string())
}

fn cmd_status(connected: &Arc<AtomicBool>) -> CommandResult {
    let status = if connected.load(Ordering::SeqCst) {
        "\x1b[32m● 已连接\x1b[0m"
    } else {
        "\x1b[31m○ 未连接\x1b[0m"
    };
    CommandResult::Success(format!("状态: {}", status))
}

fn print_banner() {
    println!(r#"
    ╔═══════════════════════════════════════════════════╗
    ║                                                   ║
    ║      ⚡ xTools CLI - 交互式串口终端 v0.1.0       ║
    ║                                                   ║
    ║           🐱 按 Tab 键补全命令 🔌                 ║
    ║                                                   ║
    ╚═══════════════════════════════════════════════════╝

    输入 'help' 查看可用命令
    "#);
}

fn print_help() {
    println!(r#"
可用命令:

  串口操作:
    list, ls              - 列出可用串口
    connect <串口> [波特率] - 连接串口 (如: connect COM3 115200)
                             ⚠️  连接后自动进入终端模式
                             ⚠️  按 Ctrl+] 退出终端模式
    disconnect, disc      - 断开串口连接
    status, st           - 查看连接状态

  数据收发:
    send <数据>          - 发送文本数据 (自动添加 \r\n)
    hex <十六进制>       - 发送十六进制数据 (如: hex 48 65 6C 6C 6F)
    terminal, term       - 手动进入交互式终端模式

  配置:
    config, cfg          - 查看/设置串口参数

  其他:
    clear, cls           - 清屏
    help, h, ?           - 显示帮助
    exit, quit, q        - 退出程序

快捷键:
    Tab                  - 命令自动补全
    Ctrl+C               - 中断/退出
    Ctrl+]               - 退出终端模式 (重要!)
    ↑/↓                  - 浏览命令历史

工作流程:
    1. xtools> list                   # 列出串口
    2. xtools> connect COM3 115200    # 连接 (自动进入终端模式)
    3. [终端模式] 直接输入交互         # 所有输入发送到串口
    4. 按 Ctrl+] 退出终端模式          # 返回命令行
    5. xtools> disconnect             # 断开连接
    6. xtools> exit                   # 退出程序
    "#);
}

// ============ 旧版 CLI（兼容保留）============

pub fn run_serial_cli(port: Option<String>, baud: u32, terminal_mode: bool) {
    println!("xTools 串口终端 v0.1.0");
    println!("========================\n");

    // 列出可用串口
    match serial::list_available_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                println!("未检测到可用串口");
                return;
            }
            println!("可用串口:");
            for (i, p) in ports.iter().enumerate() {
                println!("  [{}] {} - {}", i + 1, p.name, p.description);
            }
            println!();
        }
        Err(e) => {
            eprintln!("获取串口列表失败: {}", e);
            return;
        }
    }

    // 确定要使用的串口
    let port_name = match port {
        Some(p) => p,
        None => {
            print!("请输入串口名称 (如 COM3): ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        }
    };

    if port_name.is_empty() {
        eprintln!("未指定串口");
        return;
    }

    // 连接串口
    let mut manager = SerialManager::new();
    if let Err(e) = manager.connect(&port_name, baud, 8, 1, "none") {
        eprintln!("连接失败: {}", e);
        return;
    }

    println!("已连接到 {} @ {} bps", port_name, baud);
    println!("模式: {}", if terminal_mode { "终端交互" } else { "普通" });
    println!("提示: 按 Ctrl+C 退出\n");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // 设置 Ctrl+C 处理
    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\n收到中断信号，正在退出...");
    });

    // 简单的数据接收循环
    let _stdin = io::stdin();
    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // 读取数据
        match manager.read_available() {
            Ok(entries) => {
                for entry in entries {
                    println!("[{}] RX: {}", entry.timestamp, entry.data.trim());
                }
            }
            Err(_) => {}
        }

        thread::sleep(Duration::from_millis(50));
    }

    println!("\n已断开连接");
}
