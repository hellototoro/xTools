# xTools AI Agents 开发指南

## 项目定位

xTools 是一个跨平台串口工具，基于 Tauri 2、Vue 3、TypeScript 和 Rust 构建。

项目有两种工作模式：

- `Terminal` 终端交互模式：键盘输入直接发送到串口，串口返回内容原样写入终端。
- `Monitor` 监视器模式：持续监控串口 RX/TX 日志，支持时间戳、HEX 显示、搜索、清空、保存日志，并允许文本/HEX 发送。

项目有两种呈现形式：

- GUI：`xtools`
- CLI：`xtools_cli`

因此整体能力是“四入口，两模式”：GUI/CLI 都需要支持 Terminal 和 Monitor。

## 技术栈

### 前端

- Vue 3
- TypeScript
- Vite
- xterm.js
- Tauri JavaScript API

### 后端

- Tauri 2
- Rust
- serialport
- rustyline
- crossterm
- clap
- chrono
- dirs
- parking_lot

## 项目结构

```text
xTools/
├── src/
│   ├── App.vue              # GUI 主界面，包含 Terminal/Monitor 两种视图
│   └── main.ts              # Vue 入口
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # GUI 二进制入口：xtools
│   │   ├── main_cli.rs      # CLI 二进制入口：xtools_cli
│   │   ├── lib.rs           # Tauri command 注册和 AppState
│   │   ├── serial.rs        # 串口底层能力、事件、HEX、换行工具
│   │   ├── session.rs       # WorkMode、SerialSession、SessionState
│   │   ├── config.rs        # 配置结构、加载和保存
│   │   └── cli.rs           # CLI REPL、terminal、monitor 实现
│   ├── capabilities/
│   │   └── default.json     # Tauri 2 权限
│   ├── Cargo.toml           # Rust crate、依赖、双二进制
│   └── tauri.conf.json      # Tauri 应用和打包配置
├── package.json             # 前端依赖和脚本
├── README.md                # 用户文档
└── AGENTS.md                # 本文件
```

## 架构约定

### 串口层：`serial.rs`

`serial.rs` 只负责底层串口能力和数据格式工具：

- 枚举串口：`list_available_ports`
- 连接/断开：`SerialManager::connect_with_config`、`disconnect`
- 发送/接收：`send_payload`、`read_available`
- 数据契约：`SerialEvent`、`SerialConnectionConfig`、`SendPayload`
- 工具函数：`parse_hex_string`、`bytes_to_hex_string`、`apply_newline`

不要把 GUI、CLI、模式切换、配置持久化逻辑放进 `serial.rs`。

### 会话层：`session.rs`

`session.rs` 是 Terminal/Monitor 的共享会话抽象：

- `WorkMode::{Terminal, Monitor}`
- `SerialSession`
- `SessionState`

GUI 和 Tauri command 需要通过 `SerialSession` 管理模式和连接状态。新增模式相关行为时，优先放在会话层，而不是散落到前端或 CLI。

### Tauri 层：`lib.rs`

`lib.rs` 负责暴露给前端的 commands：

- `list_ports`
- `connect_serial`
- `disconnect_serial`
- `send_data`
- `read_serial_events`
- `is_connected`
- `get_session_state`
- `set_work_mode`
- `get_config`
- `save_config`
- `save_log`

Tauri command 应保持薄层：参数转换、锁定状态、调用 session/config/serial，不承载复杂业务逻辑。

### 配置层：`config.rs`

当前配置结构按语义拆分：

- `serial`：端口、波特率、数据位、停止位、校验位。
- `mode`：GUI 默认模式，`terminal` 或 `monitor`。
- `monitor`：HEX、换行策略、自动滚动、时间戳、HEX 显示、字体大小。
- `terminal`：终端字体大小、Ctrl+C 复制选区行为。

配置路径：

- Windows：`%APPDATA%/xtools/config.json`
- macOS：`~/Library/Application Support/xtools/config.json`
- Linux：`~/.config/xtools/config.json`

历史记录路径：

- Windows：`%APPDATA%/xtools/history.txt`
- macOS：`~/Library/Application Support/xtools/history.txt`
- Linux：`~/.config/xtools/history.txt`

旧配置不需要兼容迁移；解析失败时允许回退默认配置。

### CLI 层：`main_cli.rs` 和 `cli.rs`

CLI 子命令：

```bash
xtools_cli list
xtools_cli terminal <port> [baud]
xtools_cli monitor <port> [baud]
xtools_cli repl
xtools_cli
```

不带参数默认进入 REPL。

REPL 内命令：

```text
list
connect <port> [baud]
disconnect
monitor
terminal
mode terminal|monitor
send <data>
hex <data>
status
clear
help
exit
```

CLI 行为约定：

- `terminal` 模式使用 crossterm raw mode，键盘输入直接发串口。
- REPL 中进入 terminal 后，使用 `Ctrl+]` 返回 REPL。
- `monitor` 模式持续输出 RX 日志。
- `send` 默认追加 CRLF。
- `hex` 按 HEX 字符串发送。

### GUI 层：`App.vue`

GUI 保持单主界面：

- 左侧配置区包含串口设置和唯一的 Terminal/Monitor 模式切换。
- 主区域不要再放第二个模式切换。
- Terminal 视图使用 xterm.js。
- Monitor 视图使用日志列表和发送区。
- 串口连接配置在两种模式之间共享。

前端调用 Tauri command 时使用当前契约：

```ts
await invoke("connect_serial", { config: config.value.serial });
await invoke("send_data", { payload: { data, hex_mode } });
await invoke("read_serial_events");
await invoke("set_work_mode", { mode: config.value.mode });
```

## 开发工作流

### 安装依赖

```bash
npm install
```

### GUI 开发

```bash
npm run tauri dev
```

### CLI 开发

```bash
cd src-tauri
cargo run --bin xtools_cli -- list
cargo run --bin xtools_cli -- terminal COM3 115200
cargo run --bin xtools_cli -- monitor COM3 115200
cargo run --bin xtools_cli
```

### 构建

```bash
npm run build
npm run tauri build
```

## 验证要求

常规代码修改后至少运行：

```bash
cd src-tauri
cargo check
cargo test
```

前端或 Tauri command 契约修改后运行：

```bash
npm run build
```

真实串口行为需要设备或虚拟串口手测。无设备时不要伪造端到端串口结论。

建议手测清单：

1. `xtools_cli list`
2. `xtools_cli terminal COMx 115200`
3. `xtools_cli monitor COMx 115200`
4. GUI 刷新端口、连接、断开
5. GUI Terminal 输入和 RX 显示
6. GUI Monitor RX/TX 日志、HEX 发送、保存日志

## AI Agents 修改原则

1. 优先保持现有架构边界：serial、session、config、Tauri command、CLI、GUI 分层清晰。
2. 不要把模式逻辑复制到多个地方；能复用 `WorkMode` 和 `SerialSession` 就复用。
3. 修改 Tauri command 参数时，同步更新 `App.vue` 和相关文档。
4. 修改 CLI 子命令时，同步更新 `main_cli.rs` 测试和 README/AGENTS 说明。
5. 修改配置结构时，同步更新 Rust `AppConfig`、前端 `AppConfig` interface 和默认值。
6. 保持 GUI 深色风格和现有布局习惯，除非明确要求重做视觉设计。
7. 不要引入多串口、协议解析、脚本自动化等大功能，除非任务明确要求。
8. 不要删除用户已有改动；遇到无关脏文件时忽略，遇到冲突时先理解再处理。

## 代码风格

### Rust

- 使用 `Result<T, String>` 返回可展示错误。
- 串口读写需要考虑超时、断开、空读。
- 共享状态使用 `Arc<Mutex<T>>` 或现有项目模式。
- CLI raw mode 必须确保退出时调用 `disable_raw_mode`。
- 为工具函数、配置默认值、CLI 解析补单元测试。

### Vue

- 使用 Composition API 和 `<script setup lang="ts">`。
- 前端类型要和 Rust serde 字段保持一致，尤其是 snake_case 字段。
- Terminal 输入只在 Terminal 模式直接转发。
- Monitor 模式可以发送数据，但不能把所有键盘输入直接转发。
- 避免在主区域和配置区域重复放模式切换。

### Tauri 2

- 权限在 `src-tauri/capabilities/default.json` 中配置。
- 不要在 `tauri.conf.json` 的 `plugins.fs` 中添加旧版 `scope` 字段。
- 前端 invoke 参数名需要匹配 Rust command 的 serde/camelCase 转换规则；复杂对象优先整体传入。

## 常见问题

### 串口连接失败

常见原因：

- 串口被其他程序占用。
- 串口名称错误。
- 波特率或参数不匹配。
- Linux/macOS 权限不足。

Linux 可检查：

```bash
sudo usermod -a -G dialout $USER
ls /dev/tty*
```

Windows 可检查：

```cmd
mode
```

### GUI 运行后显示 localhost 拒绝连接

通常是前端资源未正确构建或 Tauri dev server 未启动。

```bash
npm run build
npm run tauri dev
```

### rustyline 历史记录不生效

检查配置目录是否可写，并确认 `xtools/history.txt` 能创建。

## 发布产物

Windows 常见产物：

- `xtools.exe`：GUI 版本。
- `xtools_cli.exe`：CLI 版本。
- `xTools_x.x.x_x64-setup.exe`：NSIS 安装包。

macOS/Linux 产物由 Tauri bundle 配置决定。

## 提交规范

使用 Conventional Commits：

- `feat:` 新功能
- `fix:` 修复 bug
- `docs:` 文档更新
- `refactor:` 重构
- `test:` 测试
- `chore:` 工具或构建

示例：

```text
refactor(serial): split session mode from serial manager
fix(gui): keep mode switch only in sidebar
docs(agents): update architecture guide
```
