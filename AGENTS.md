# xTools AI Agents 开发指南

## 概述

本文档记录了 xTools 项目开发过程中使用的 AI agents 配置和最佳实践。

## 项目结构

```
xTools/
├── src/                    # 前端源码 (Vue 3 + TypeScript)
│   ├── App.vue            # 主界面组件
│   └── main.ts            # 应用入口
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # GUI 入口，启动 Tauri 应用
│   │   ├── main_cli.rs    # CLI 入口，交互式 REPL
│   │   ├── lib.rs         # Tauri 命令处理和共享逻辑
│   │   ├── serial.rs      # 串口通信核心逻辑
│   │   ├── config.rs      # 配置管理和持久化
│   │   └── cli.rs         # CLI 交互实现（rustyline）
│   ├── icons/             # 应用图标资源
│   ├── Cargo.toml         # Rust 依赖配置（定义两个二进制文件）
│   ├── tauri.conf.json    # Tauri 应用配置
│   └── capabilities/      # Tauri 2 权限配置
│       └── default.json   # 默认权限（fs, shell）
├── package.json           # Node.js 依赖配置
├── agents.md              # AI Agents 开发指南
└── README.md              # 项目说明文档
```

## 技术栈

### 前端
- **Vue 3**: 渐进式 JavaScript 框架
- **TypeScript**: 类型安全
- **Vite**: 快速构建工具

### 后端
- **Tauri 2**: 跨平台桌面应用框架
- **Rust**: 系统级编程语言
- **serialport**: 串口通信库
- **rustyline**: 交互式命令行库（Tab 补全、历史记录）
- **tokio**: 异步运行时
- **chrono**: 时间处理
- **dirs**: 跨平台目录路径

## 核心功能模块

### 1. 串口管理 (`serial.rs`)

**职责**：
- 串口设备枚举
- 连接/断开串口
- 数据发送/接收
- 十六进制模式支持

**关键 API**：
```rust
pub fn list_available_ports() -> Result<Vec<PortInfo>, String>
pub fn connect(&mut self, port_name: &str, baud_rate: u32, ...) -> Result<(), String>
pub fn send(&mut self, data: &str, hex_mode: bool) -> Result<(), String>
pub fn read_available(&mut self) -> Result<Vec<DataEntry>, String>
```

### 2. 配置管理 (`config.rs`)

**职责**：
- 应用配置持久化
- 用户偏好保存
- 配置加载/保存

**配置路径**：
- Windows: `%APPDATA%/xtools/config.json`
- macOS: `~/Library/Application Support/xtools/config.json`
- Linux: `~/.config/xtools/config.json`

**历史记录路径**：
- Windows: `%APPDATA%/xtools/history.txt`
- macOS: `~/Library/Application Support/xtools/history.txt`
- Linux: `~/.config/xtools/history.txt`

### 3. CLI 交互模式 (`cli.rs` + `main_cli.rs`)

**职责**：
- 交互式 REPL（Read-Eval-Print Loop）
- Tab 键命令补全
- 命令历史记录（持久化）
- 串口命令执行

**支持的命令**：
- `list` - 列出可用串口
- `connect <port> [baud]` - 连接串口
- `disconnect` - 断开连接
- `send <data>` - 发送数据（文本模式）
- `hex <data>` - 发送十六进制数据
- `config [key] [value]` - 查看/设置配置
- `status` - 查看当前状态
- `clear` - 清空屏幕
- `help` - 显示帮助
- `exit` / `quit` - 退出

**使用示例**：
```bash
# 启动 GUI 模式
xtools.exe

# 启动交互式 CLI
xtools_cli.exe

# 在 CLI 中操作
xtools> list
xtools> connect COM3 115200
xtools> send Hello World
xtools> hex 48 65 6C 6C 6F
xtools> disconnect
```

**Tab 补全功能**：
- 按 Tab 键可自动补全命令
- 支持多级补全（命令 → 参数）
- 显示命令提示和说明

### 4. 前端界面 (`App.vue`)

**功能区域**：
1. **侧边栏** - 串口配置和显示设置
2. **终端区** - 数据显示和交互
3. **工具栏** - 清空、保存日志、搜索
4. **发送区** - 数据输入和发送（非终端模式）

## 开发工作流

### 1. 环境准备

```bash
# 安装依赖
npm install

# 检查 Rust 环境
rustc --version
cargo --version
```

### 2. 开发模式

```bash
# 启动开发服务器（热重载）
npm run tauri dev
```

### 3. 构建发布

```bash
# 构建前端资源
npm run build

# 构建 Tauri 应用（自动构建两个二进制文件）
npm run tauri build

# 输出位置
# Windows:
#   - xtools.exe          (~10 MB, GUI 模式，包含 WebView)
#   - xtools_cli.exe      (~780 KB, CLI 模式，纯终端)
#   - xTools_x.x.x_x64-setup.exe  (NSIS 安装程序)
#   - xTools_x.x.x_x64_en-US.msi  (MSI 安装包)
#
# macOS:
#   - xTools.app
#   - xtools_cli
#   - xTools_x.x.x_x64.dmg
#
# Linux:
#   - xtools
#   - xtools_cli
#   - xtools_x.x.x_amd64.deb
#   - xtools_x.x.x_amd64.AppImage
```

### 4. 测试运行

```bash
# 直接运行可执行文件
.\src-tauri\target\release\xtools.exe      # GUI 模式
.\src-tauri\target\release\xtools_cli.exe  # CLI 模式

# 或使用开发模式
npm run tauri dev
```

## AI Agents 使用建议

### 代码修改原则

1. **最小化修改**：只改必要的代码
2. **保持风格一致**：遵循现有代码风格
3. **避免破坏性修改**：不删除正常工作的代码
4. **测试验证**：修改后运行测试确认

### Rust 开发注意事项

- 使用 `Result<T, String>` 处理错误
- 使用 `Arc<Mutex<T>>` 共享状态
- 注意异步上下文（tokio runtime）
- 串口操作需要考虑超时和错误处理
- **两个独立二进制**：
  - `main.rs` → `xtools` (GUI)
  - `main_cli.rs` → `xtools_cli` (CLI)
  - 共享逻辑在 `lib.rs` 中
- **rustyline 集成**：
  - 实现 `Completer` trait 提供 Tab 补全
  - 实现 `Hinter` trait 提供命令提示
  - 使用 `Editor::readline()` 读取输入

### Vue 开发注意事项

- 使用 Composition API (`<script setup>`)
- 响应式数据使用 `ref` 和 `reactive`
- 事件处理使用 `@` 语法糖
- 样式使用 CSS 变量统一主题

### Tauri 2 特性

- **前端调用**：`invoke('command_name', { param: value })`
- **后端定义**：`#[tauri::command]`
- **状态管理**：使用 `State<T>` 访问共享状态
- **权限系统**：Tauri 2 使用 capabilities 配置权限
  - `capabilities/default.json` 定义文件系统权限
  - 不再在 `tauri.conf.json` 中配置 `scope`
- **插件系统**：使用 `tauri_plugin_fs`、`tauri_plugin_shell` 等
- **多二进制支持**：在 `Cargo.toml` 中定义多个 `[[bin]]`

## 常见问题

### 1. 串口连接失败

**原因**：
- 串口被其他程序占用
- 权限不足（Linux/macOS）
- 串口参数不正确

**解决**：
```bash
# Linux 添加用户到 dialout 组
sudo usermod -a -G dialout $USER

# 检查串口是否存在
# Linux/macOS
ls /dev/tty*

# Windows
mode
```

### 2. Tauri 2 fs 插件配置错误

**错误信息**：
```
PluginInitialization("fs", "Error deserializing 'plugins.fs' within your Tauri configuration: unknown field `scope`")
```

**解决**：
- Tauri 2 不再在 `tauri.conf.json` 的 `plugins.fs` 中配置 `scope`
- 改为在 `capabilities/default.json` 中配置权限
- 参考示例：
```json
{
  "permissions": [
    "fs:allow-read",
    "fs:allow-write",
    "shell:allow-open"
  ]
}
```

### 3. 运行 xtools.exe 显示 localhost 拒绝连接

**原因**：
- 前端资源未正确打包到可执行文件中
- `npm run tauri build` 之前需要先构建前端

**解决**：
```bash
# 确保先构建前端
npm run build

# 然后构建 Tauri
npm run tauri build
```

### 4. 编译错误

**常见原因**：
- Rust 版本过低：需要 1.70+
- 依赖冲突：删除 `Cargo.lock` 重新构建
- Node 版本问题：需要 Node 18+

**解决**：
```bash
# 更新 Rust
rustup update

# 清理并重新构建
cargo clean
npm run tauri build
```

### 5. 前端热重载不工作

**解决**：
```bash
# 清理缓存
rm -rf node_modules dist
npm install
npm run tauri dev
```

### 6. rustyline 历史记录不生效

**检查**：
- 确保配置目录有写权限
- 历史文件路径：`~/.config/xtools/history.txt`
- 可手动创建目录：`mkdir -p ~/.config/xtools`

## 架构设计

### 双二进制架构

项目采用双二进制设计，分离 GUI 和 CLI：

**优势**：
- ✅ 独立部署：CLI 版本可单独分发（仅 780KB）
- ✅ 资源优化：CLI 不包含 WebView，启动更快
- ✅ 共享逻辑：核心功能在 `lib.rs` 中复用
- ✅ 灵活使用：用户可根据场景选择合适的版本

**Cargo.toml 配置**：
```toml
[[bin]]
name = "xtools"
path = "src/main.rs"

[[bin]]
name = "xtools_cli"
path = "src/main_cli.rs"
```

### 串口通信架构

```
┌─────────────────┐
│   Vue Frontend  │
└────────┬────────┘
         │ invoke()
    ┌────▼─────────────┐
    │  Tauri Commands  │
    │    (lib.rs)      │
    └────┬─────────────┘
         │
    ┌────▼─────────────┐
    │  SerialManager   │
    │   (serial.rs)    │
    └────┬─────────────┘
         │
    ┌────▼─────────────┐
    │ serialport-rs    │
    └──────────────────┘
```

### CLI REPL 架构

```
┌─────────────────┐
│  main_cli.rs    │ ──┐
└─────────────────┘   │
                      │
┌─────────────────┐   │
│    cli.rs       │ ◀─┘
│  ┌───────────┐  │
│  │ rustyline │  │
│  └───────────┘  │
└────┬────────────┘
     │
┌────▼────────────┐
│ SerialManager   │
└─────────────────┘
```

## 扩展功能建议

### 未来可添加的功能

1. **多串口支持**：同时管理多个串口连接
2. **数据过滤**：按关键词/正则表达式过滤显示
3. **脚本自动化**：支持发送脚本序列和定时任务
4. **波形显示**：数据可视化（折线图、示波器模式）
5. **插件系统**：支持扩展功能（自定义协议解析）
6. **数据录制回放**：记录并重放串口数据流
7. **协议分析**：常见协议解析（Modbus、AT 命令等）
8. **终端仿真**：支持 ANSI 转义序列和颜色

### 代码优化方向

1. **性能优化**：
   - 使用虚拟滚动优化大量数据显示
   - 后台线程处理串口数据
   - 数据缓冲区优化

2. **用户体验**：
   - 添加快捷键配置
   - 主题切换（明/暗/自定义）
   - 国际化支持（i18n）
   - 窗口布局保存

3. **测试覆盖**：
   - 单元测试（Rust/TypeScript）
   - 集成测试
   - E2E 测试（使用 Playwright）
   - 虚拟串口测试

4. **CLI 增强**：
   - 支持批处理脚本
   - 添加配置文件支持
   - 日志输出到文件
   - 彩色输出支持

## 技术细节

### Tauri 2 权限配置

**capabilities/default.json**：
```json
{
  "identifier": "default",
  "description": "Default permissions",
  "permissions": [
    "core:default",
    "fs:allow-read",
    "fs:allow-write",
    "fs:allow-exists",
    "shell:allow-open"
  ]
}
```

### 图标生成

项目使用用户提供的 Totoro 图片生成了完整的图标集：

**包含的尺寸**：
- 32x32.png
- 128x128.png
- 128x128@2x.png
- icon.png (512x512)
- icon.ico (Windows)
- icon.icns (macOS，需手动转换)

**生成工具**：
- `sharp` - 图片缩放
- `to-ico` - 生成 .ico 文件

### 构建产物

| 文件 | 大小 | 说明 |
|------|------|------|
| xtools.exe | ~10 MB | GUI 版本（包含 WebView2） |
| xtools_cli.exe | ~780 KB | CLI 版本（纯终端） |
| xTools_x.x.x_x64-setup.exe | ~2 MB | NSIS 安装程序 |
| xTools_x.x.x_x64_en-US.msi | ~3.5 MB | MSI 安装包 |

## 参考资源

### 官方文档
- [Tauri 2 官方文档](https://tauri.app/)
- [Vue 3 文档](https://vuejs.org/)
- [Rust 官方文档](https://doc.rust-lang.org/)

### 依赖库文档
- [serialport-rs](https://github.com/serialport/serialport-rs) - 串口通信
- [rustyline](https://docs.rs/rustyline/) - 交互式命令行
- [tokio](https://tokio.rs/) - 异步运行时
- [chrono](https://docs.rs/chrono/) - 时间处理
- [dirs](https://docs.rs/dirs/) - 系统目录

### 相关工具
- [Vite](https://vitejs.dev/) - 前端构建工具
- [TypeScript](https://www.typescriptlang.org/) - 类型安全
- [Cargo](https://doc.rust-lang.org/cargo/) - Rust 包管理器

## 开发团队协作

### Git 工作流

```bash
# 克隆项目
git clone git@github.com:hellototoro/xTools.git
cd xTools

# 安装依赖
npm install

# 创建功能分支
git checkout -b feature/new-feature

# 开发并测试
npm run tauri dev

# 提交更改
git add .
git commit -m "feat: add new feature"

# 推送到远程
git push origin feature/new-feature
```

### 提交规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

- `feat:` 新功能
- `fix:` 修复 bug
- `docs:` 文档更新
- `style:` 代码格式调整
- `refactor:` 重构代码
- `test:` 测试相关
- `chore:` 构建/工具相关

**示例**：
```
feat(serial): add multi-port support
fix(cli): resolve tab completion issue
docs(readme): update installation guide
```

## 版本历史

### v0.1.0 (2026-01-24)

**首次发布** 🎉

**功能**：
- ✅ 双模式支持（GUI + CLI）
- ✅ 串口通信（连接、发送、接收）
- ✅ 十六进制模式
- ✅ 自动滚动、时间戳、日志保存
- ✅ 终端模式（CLI 交互）
- ✅ Tab 补全和命令历史
- ✅ 配置持久化
- ✅ 跨平台支持（Windows/macOS/Linux）

**技术栈**：
- Tauri 2.3
- Vue 3.5
- Rust 1.70+
- TypeScript 5.x

**已知问题**：
- [ ] macOS icon.icns 需手动转换
- [ ] 两个 Rust 编译警告（不影响使用）

## 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 如何贡献

1. **Fork 本项目**到你的 GitHub 账户
2. **克隆** Fork 的仓库到本地
3. 创建**功能分支** (`git checkout -b feature/AmazingFeature`)
4. **编写代码**并确保通过测试
5. **提交改动** (`git commit -m 'feat: add some amazing feature'`)
6. **推送分支** (`git push origin feature/AmazingFeature`)
7. 创建 **Pull Request**

### 代码审查标准

- ✅ 代码风格符合项目规范
- ✅ 包含必要的注释和文档
- ✅ 通过所有测试（如有）
- ✅ 不破坏现有功能
- ✅ 提交信息清晰明确

### 报告问题

使用 [GitHub Issues](https://github.com/hellototoro/xTools/issues) 报告：

**Bug 报告应包含**：
- 操作系统和版本
- 软件版本
- 复现步骤
- 预期行为
- 实际行为
- 错误信息或截图

**功能建议应包含**：
- 功能描述
- 使用场景
- 实现思路（可选）

## 许可证

MIT License

Copyright (c) 2026 hellototoro

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

**Made with ❤️ by hellototoro**

Repository: https://github.com/hellototoro/xTools
