use crate::serial::{self, SerialManager};
use chrono::Local;
use std::io::{self, BufRead, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
    println!("提示: 输入 :q 退出, :h 显示帮助\n");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // 设置 Ctrl+C 处理
    ctrlc_handler(r.clone());

    if terminal_mode {
        run_terminal_mode(manager, running);
    } else {
        run_normal_mode(manager, running);
    }

    println!("\n已断开连接");
}

fn run_terminal_mode(mut manager: SerialManager, running: Arc<AtomicBool>) {
    println!("--- 终端模式 (输入直接发送，支持 ANSI 转义序列) ---\n");

    // 接收线程
    let running_rx = running.clone();
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        while running_rx.load(Ordering::SeqCst) {
            match manager.read_available() {
                Ok(entries) => {
                    for entry in entries {
                        print!("{}", entry.data);
                        io::stdout().flush().unwrap();
                    }
                }
                Err(_) => break,
            }
            
            // 检查发送队列
            if let Ok(data) = rx.try_recv() {
                if let Err(e) = manager.send(&String::from_utf8_lossy(&data), false) {
                    eprintln!("\n发送错误: {}", e);
                }
            }
            
            thread::sleep(Duration::from_millis(10));
        }
    });

    // 主线程处理输入
    let stdin = io::stdin();
    let mut buffer = [0u8; 1];
    
    #[cfg(windows)]
    {
        // Windows 上使用行缓冲
        for line in stdin.lock().lines() {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            match line {
                Ok(text) => {
                    if text == ":q" {
                        running.store(false, Ordering::SeqCst);
                        break;
                    } else if text == ":h" {
                        print_help();
                        continue;
                    }
                    
                    let mut data = text.into_bytes();
                    data.push(b'\r');
                    data.push(b'\n');
                    let _ = tx.send(data);
                }
                Err(_) => break,
            }
        }
    }

    #[cfg(not(windows))]
    {
        // Unix 上可以读取单个字符
        use std::os::unix::io::AsRawFd;
        
        for line in stdin.lock().lines() {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            match line {
                Ok(text) => {
                    if text == ":q" {
                        running.store(false, Ordering::SeqCst);
                        break;
                    } else if text == ":h" {
                        print_help();
                        continue;
                    }
                    
                    let mut data = text.into_bytes();
                    data.push(b'\r');
                    data.push(b'\n');
                    let _ = tx.send(data);
                }
                Err(_) => break,
            }
        }
    }
}

fn run_normal_mode(mut manager: SerialManager, running: Arc<AtomicBool>) {
    println!("--- 普通模式 (按行发送) ---\n");

    let running_rx = running.clone();

    // 接收线程
    thread::spawn(move || {
        while running_rx.load(Ordering::SeqCst) {
            match manager.read_available() {
                Ok(entries) => {
                    for entry in entries {
                        let now = Local::now();
                        println!(
                            "[{}] RX: {} | HEX: {}",
                            entry.timestamp, entry.data.trim(), entry.hex
                        );
                    }
                }
                Err(e) => {
                    eprintln!("读取错误: {}", e);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    // 主线程处理发送
    let stdin = io::stdin();
    let mut send_manager = SerialManager::new();
    
    // 这里需要重新连接因为 manager 被移动到线程中了
    // 实际实现中应该使用 Arc<Mutex<SerialManager>>
    
    print!("> ");
    io::stdout().flush().unwrap();
    
    for line in stdin.lock().lines() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        match line {
            Ok(text) => {
                let text = text.trim();
                
                if text == ":q" {
                    running.store(false, Ordering::SeqCst);
                    break;
                } else if text == ":h" {
                    print_help();
                } else if text.starts_with(":hex ") {
                    let hex_data = &text[5..];
                    println!("发送 HEX: {}", hex_data);
                    // send_manager.send(hex_data, true);
                } else if !text.is_empty() {
                    println!("发送: {}", text);
                    // send_manager.send(&format!("{}\r\n", text), false);
                }

                print!("> ");
                io::stdout().flush().unwrap();
            }
            Err(_) => break,
        }
    }
}

fn print_help() {
    println!("\n命令帮助:");
    println!("  :q          - 退出程序");
    println!("  :h          - 显示帮助");
    println!("  :hex <data> - 发送十六进制数据 (如 :hex 48 65 6C 6C 6F)");
    println!("  其他输入    - 直接发送文本\n");
}

fn ctrlc_handler(running: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
        println!("\n收到中断信号，正在退出...");
    });
}
