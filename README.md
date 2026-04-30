# xTools - 跨平台超级工具集

基于 Tauri 2 + Vue 3 + TypeScript 构建的跨平台桌面工具集。

## 🔧 串口终端工具

### 功能特性

- **双模式运行**：支持 GUI 界面和 CLI 命令行两种运行方式
- **终端模式**：类似真实终端的交互体验
- **普通模式**：按行发送数据，适合调试
- **十六进制支持**：发送/接收 HEX 数据
- **自动滚动**：新数据自动滚动到底部
- **时间戳显示**：精确到毫秒的时间戳
- **日志保存**：导出通信日志
- **配置持久化**：自动保存用户设置

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+C` | 复制选中内容 |
| `Ctrl+V` | 粘贴 |
| `Ctrl+F` | 打开/关闭搜索 |
| `Ctrl+Enter` | 发送数据 |

## 🚀 开发

### 环境要求

- Node.js 18+
- Rust 1.70+
- [Tauri 2 Prerequisites](https://tauri.app/start/prerequisites/)

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
cargo run --bin xtools_cli - 启动 CLI 版本（在 src-tauri 目录）
```

### 构建发布

```bash
npm run tauri build
```

## 📦 使用方式

### GUI 模式

```bash
# 启动图形界面
xtools
```

### CLI 交互式终端

```bash
# 查看帮助
xtools_cli --help

# 列出可用串口（子命令）
xtools_cli list

# 直接连接串口并进入终端模式
xtools_cli connect COM3 115200

# 启动 CLI 交互终端（支持 Tab 补全）
xtools_cli

# 命令行模式 - 可用命令：
xtools> list                    # 列出串口
xtools> connect COM3 115200     # 连接串口（自动进入终端模式）
xtools> status                  # 查看状态
xtools> help                    # 查看帮助
xtools> exit                    # 退出

# ⚠️ 重要：连接串口后会自动进入终端模式
# 在终端模式下：
#   - 所有输入直接发送到串口设备
#   - 按 Ctrl+] 退出终端模式，返回命令行
#   - 退出后可使用 disconnect 命令断开连接

# 完整工作流程：
xtools> list                    # 1. 列出可用串口
xtools> connect COM3 115200     # 2. 连接串口（进入终端模式）
[终端模式] 直接输入与串口交互     # 3. 直接输入数据
[按 Ctrl+] 退出]                # 4. 退出终端模式
xtools> disconnect              # 5. 断开串口连接
xtools> exit                    # 6. 退出程序

# 快捷键：
#   Tab      - 命令自动补全
#   ↑/↓      - 浏览历史命令
#   Ctrl+C   - 中断/退出程序
#   Ctrl+]   - 退出终端模式（重要！）
```

Windows 安装包会在安装时把 `xtools_cli.exe` 所在目录追加到当前用户的 `PATH`，安装完成后重新打开终端即可直接执行 `xtools_cli`。

## 📁 项目结构

```
xTools/
├── src/                    # 前端源码 (Vue 3)
│   ├── App.vue            # 主界面
│   └── main.ts            # 入口
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # CLI 入口
│   │   ├── lib.rs         # Tauri 命令
│   │   ├── serial.rs      # 串口管理
│   │   ├── config.rs      # 配置管理
│   │   └── cli.rs         # CLI 交互
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## 📝 License

MIT
