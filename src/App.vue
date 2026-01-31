<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

// Types
interface PortInfo {
  name: string;
  description: string;
}

interface DataEntry {
  timestamp: string;
  data: string;
  hex: string;
  direction: string;
}

interface SerialConfig {
  port: string;
  baud_rate: number;
  custom_baud_rate: number;
  data_bits: number;
  stop_bits: number;
  parity: string;
  hex_mode: boolean;
  append_newline: boolean;
  newline_type: string;
}

interface DisplayConfig {
  auto_scroll: boolean;
  show_timestamp: boolean;
  show_hex: boolean;
  font_size: number;
  terminal_mode: boolean;
}

interface AppConfig {
  serial: SerialConfig;
  display: DisplayConfig;
}

// State
const ports = ref<PortInfo[]>([]);
const connected = ref(false);
const dataLog = ref<DataEntry[]>([]);
const sendText = ref("");
const searchText = ref("");
const showSearch = ref(false);
const searchIndex = ref(-1);
const customBaudRate = ref(false);

const config = ref<AppConfig>({
  serial: {
    port: "",
    baud_rate: 115200,
    custom_baud_rate: 0,
    data_bits: 8,
    stop_bits: 1,
    parity: "none",
    hex_mode: false,
    append_newline: true,
    newline_type: "crlf",
  },
  display: {
    auto_scroll: true,
    show_timestamp: true,
    show_hex: false,
    font_size: 14,
    terminal_mode: false,
  },
});

const baudRates = [300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
const parityOptions = ["none", "odd", "even"];
const newlineOptions = [
  { value: "crlf", label: "CRLF (\\r\\n)" },
  { value: "lf", label: "LF (\\n)" },
  { value: "cr", label: "CR (\\r)" },
];

let pollInterval: number | null = null;
const terminalRef = ref<HTMLDivElement | null>(null);
const xtermContainerRef = ref<HTMLDivElement | null>(null);
let xterm: Terminal | null = null;
let fitAddon: FitAddon | null = null;

// Computed
const filteredLog = computed(() => {
  if (!searchText.value) return dataLog.value;
  const search = searchText.value.toLowerCase();
  return dataLog.value.filter(
    (entry) =>
      entry.data.toLowerCase().includes(search) ||
      entry.hex.toLowerCase().includes(search)
  );
});

// Methods
async function refreshPorts() {
  try {
    ports.value = await invoke<PortInfo[]>("list_ports");
  } catch (e) {
    console.error("获取串口列表失败:", e);
  }
}

async function connect() {
  try {
    await invoke("connect_serial", {
      port: config.value.serial.port,
      baudRate: config.value.serial.baud_rate,
      dataBits: config.value.serial.data_bits,
      stopBits: config.value.serial.stop_bits,
      parity: config.value.serial.parity,
    });
    connected.value = true;
    startPolling();
    await saveConfig();
  } catch (e: any) {
    alert("连接失败: " + e);
  }
}

async function disconnect() {
  try {
    stopPolling();
    await invoke("disconnect_serial");
    connected.value = false;
  } catch (e: any) {
    console.error("断开失败:", e);
  }
}

function startPolling() {
  if (pollInterval) return;
  pollInterval = window.setInterval(async () => {
    try {
      const entries = await invoke<DataEntry[]>("read_data");
      if (entries.length > 0) {
        dataLog.value.push(...entries);
        // 写入 xterm 终端
        for (const entry of entries) {
          if (entry.direction === 'rx') {
            writeToXterm(entry.data);
          }
        }
        if (config.value.display.auto_scroll) {
          scrollToBottom();
        }
      }
    } catch (e) {
      console.error("读取数据失败:", e);
    }
  }, 50);
}

function stopPolling() {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
}

async function send() {
  if (!sendText.value || !connected.value) return;

  let data = sendText.value;
  if (config.value.serial.append_newline) {
    switch (config.value.serial.newline_type) {
      case "crlf":
        data += "\r\n";
        break;
      case "lf":
        data += "\n";
        break;
      case "cr":
        data += "\r";
        break;
    }
  }

  try {
    await invoke("send_data", {
      data,
      hexMode: config.value.serial.hex_mode,
    });

    // 添加到日志
    const now = new Date();
    const timestamp = now.toTimeString().split(" ")[0] + "." + now.getMilliseconds().toString().padStart(3, "0");
    dataLog.value.push({
      timestamp,
      data: sendText.value,
      hex: config.value.serial.hex_mode ? sendText.value : stringToHex(sendText.value),
      direction: "tx",
    });

    if (!config.value.display.terminal_mode) {
      sendText.value = "";
    }
    if (config.value.display.auto_scroll) {
      scrollToBottom();
    }
  } catch (e: any) {
    alert("发送失败: " + e);
  }
}

function stringToHex(str: string): string {
  return Array.from(str)
    .map((c) => c.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0"))
    .join(" ");
}

function scrollToBottom() {
  nextTick(() => {
    if (terminalRef.value) {
      terminalRef.value.scrollTop = terminalRef.value.scrollHeight;
    }
  });
}

function clearLog() {
  dataLog.value = [];
  if (xterm) {
    xterm.clear();
  }
}

// 初始化 xterm.js 终端
function initXterm() {
  if (xterm || !xtermContainerRef.value) return;
  
  xterm = new Terminal({
    fontSize: config.value.display.font_size,
    fontFamily: '"Cascadia Code", "Fira Code", Consolas, monospace',
    theme: {
      background: '#1a1a2e',
      foreground: '#e4e4e7',
      cursor: '#a78bfa',
      cursorAccent: '#1a1a2e',
      selectionBackground: 'rgba(167, 139, 250, 0.3)',
    },
    cursorBlink: true,
    convertEol: true,
    allowProposedApi: true,
  });
  
  fitAddon = new FitAddon();
  xterm.loadAddon(fitAddon);
  xterm.open(xtermContainerRef.value);
  fitAddon.fit();
  
  // 监听用户输入，发送到串口
  xterm.onData(async (data) => {
    if (!connected.value) return;
    try {
      await invoke("send_data", { data, hexMode: false });
    } catch (err) {
      console.error("发送失败:", err);
    }
  });
  
  // 自定义键盘事件处理器 - 在 xterm 处理之前拦截特殊快捷键
  xterm.attachCustomKeyEventHandler((e) => {
    // Ctrl+C - 复制选中文本
    if (e.ctrlKey && e.key === 'c' && e.type === 'keydown') {
      const selection = xterm?.getSelection();
      if (selection && selection.trim().length > 0) {
        e.preventDefault();
        navigator.clipboard.writeText(selection).then(() => {
          xterm?.clearSelection();
          console.log('已复制到剪贴板:', selection);
        }).catch(err => {
          console.error('复制失败:', err);
        });
        return false; // 阻止 xterm 处理此事件
      }
      // 没有选中文本，让 xterm 处理 Ctrl+C（发送中断信号）
      return true;
    }
    
    // Ctrl+V - 粘贴
    if (e.ctrlKey && e.key === 'v' && e.type === 'keydown') {
      e.preventDefault();
      navigator.clipboard.readText().then(async (text) => {
        if (text) {
          try {
            await invoke("send_data", { data: text, hexMode: false });
            console.log('已粘贴:', text);
          } catch (err) {
            console.error('粘贴失败:', err);
          }
        }
      }).catch(err => {
        console.error('读取剪贴板失败:', err);
      });
      return false; // 阻止 xterm 处理此事件
    }
    
    // 其他按键让 xterm 正常处理
    return true;
  });
  
  // 监听选择变化事件（用于调试）
  xterm.onSelectionChange(() => {
    if (!xterm) return;
    const selection = xterm.getSelection();
    if (selection && selection.trim().length > 0) {
      console.log('选中了文本:', selection);
    }
  });
  
  // 监听窗口大小变化
  window.addEventListener('resize', handleResize);
}

function handleResize() {
  if (fitAddon && xterm) {
    fitAddon.fit();
  }
}

function disposeXterm() {
  window.removeEventListener('resize', handleResize);
  if (xterm) {
    xterm.dispose();
    xterm = null;
  }
  fitAddon = null;
}

// 写入数据到 xterm
function writeToXterm(data: string) {
  if (xterm && config.value.display.terminal_mode) {
    xterm.write(data);
  }
}

async function saveLog() {
  const content = dataLog.value
    .map((entry) => {
      const dir = entry.direction === "tx" ? "TX" : "RX";
      const ts = config.value.display.show_timestamp ? `[${entry.timestamp}] ` : "";
      const hex = config.value.display.show_hex ? ` | HEX: ${entry.hex}` : "";
      return `${ts}${dir}: ${entry.data}${hex}`;
    })
    .join("\n");

  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const filename = `serial_log_${timestamp}.txt`;

  try {
    await invoke("save_log", { path: filename, content });
    alert(`日志已保存: ${filename}`);
  } catch (e: any) {
    alert("保存失败: " + e);
  }
}

async function loadConfig() {
  try {
    const cfg = await invoke<AppConfig>("get_config");
    config.value = cfg;
  } catch (e) {
    console.error("加载配置失败:", e);
  }
}

async function saveConfig() {
  try {
    await invoke("save_config", { config: config.value });
  } catch (e) {
    console.error("保存配置失败:", e);
  }
}

// 自动保存配置
watch(config, () => saveConfig(), { deep: true });

// Keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === "f") {
    e.preventDefault();
    showSearch.value = !showSearch.value;
    if (!showSearch.value) {
      searchText.value = "";
    }
  }
  // 全局 Ctrl+C 处理（用于终端区域的文本选择）
  if (e.ctrlKey && (e.key === "c" || e.key === "C")) {
    const selection = window.getSelection();
    const selectedText = selection ? selection.toString() : '';
    if (selectedText.length > 0) {
      e.preventDefault();
      navigator.clipboard.writeText(selectedText);
      selection?.removeAllRanges();
    }
  }
}

function findNext() {
  if (!searchText.value || filteredLog.value.length === 0) return;
  searchIndex.value = (searchIndex.value + 1) % filteredLog.value.length;
}

function findPrev() {
  if (!searchText.value || filteredLog.value.length === 0) return;
  searchIndex.value = searchIndex.value <= 0 ? filteredLog.value.length - 1 : searchIndex.value - 1;
}

// Lifecycle
onMounted(async () => {
  await loadConfig();
  await refreshPorts();
  document.addEventListener("keydown", handleKeydown);
  // 如果启动时就是终端模式，初始化 xterm
  if (config.value.display.terminal_mode) {
    nextTick(() => initXterm());
  }
});

onUnmounted(() => {
  stopPolling();
  disposeXterm();
  document.removeEventListener("keydown", handleKeydown);
});

// 监听终端模式切换
watch(() => config.value.display.terminal_mode, (newVal) => {
  if (newVal) {
    nextTick(() => initXterm());
  } else {
    disposeXterm();
  }
});

// 监听字体大小变化
watch(() => config.value.display.font_size, (newVal) => {
  if (xterm) {
    xterm.options.fontSize = newVal;
    if (fitAddon) fitAddon.fit();
  }
});
</script>

<template>
  <div class="app">
    <!-- 顶部工具栏 -->
    <header class="toolbar">
      <div class="logo">
        <span class="icon">⚡</span>
        <span>xTools 串口终端</span>
      </div>
      <div class="status" :class="{ connected }">
        {{ connected ? "● 已连接" : "○ 未连接" }}
      </div>
    </header>

    <div class="main-content">
      <!-- 左侧设置面板 -->
      <aside class="sidebar">
        <section class="panel">
          <h3>串口设置</h3>
          
          <div class="form-group">
            <label>端口</label>
            <div class="port-select">
              <select v-model="config.serial.port" :disabled="connected">
                <option value="">选择串口...</option>
                <option v-for="p in ports" :key="p.name" :value="p.name">
                  {{ p.name }} - {{ p.description }}
                </option>
              </select>
              <button class="btn-icon" @click="refreshPorts" title="刷新">🔄</button>
            </div>
          </div>

          <div class="form-group">
            <label>波特率</label>
            <div class="baud-select">
              <select
                :value="customBaudRate ? -1 : (baudRates.includes(config.serial.baud_rate) ? config.serial.baud_rate : -1)"
                :disabled="connected"
                @change="e => { const v = parseInt((e.target as HTMLSelectElement).value); if (v === -1) { customBaudRate = true; if (config.serial.custom_baud_rate > 0) config.serial.baud_rate = config.serial.custom_baud_rate; } else { config.serial.baud_rate = v; customBaudRate = false; } }"
              >
                <option v-for="b in baudRates" :key="b" :value="b">{{ b }}</option>
                <option :value="-1">自定义...</option>
              </select>
            </div>
            <input
              v-if="customBaudRate || !baudRates.includes(config.serial.baud_rate)"
              class="custom-baud-input"
              type="text"
              :value="config.serial.baud_rate > 0 && !baudRates.includes(config.serial.baud_rate) ? config.serial.baud_rate : (config.serial.custom_baud_rate > 0 ? config.serial.custom_baud_rate : '')"
              @input="e => { const v = parseInt((e.target as HTMLInputElement).value); if (!isNaN(v) && v > 0) { config.serial.baud_rate = v; config.serial.custom_baud_rate = v; } }"
              :disabled="connected"
              placeholder="输入自定义波特率"
            />
          </div>

          <div class="form-row">
            <div class="form-group">
              <label>数据位</label>
              <select v-model="config.serial.data_bits" :disabled="connected">
                <option :value="5">5</option>
                <option :value="6">6</option>
                <option :value="7">7</option>
                <option :value="8">8</option>
              </select>
            </div>
            <div class="form-group">
              <label>停止位</label>
              <select v-model="config.serial.stop_bits" :disabled="connected">
                <option :value="1">1</option>
                <option :value="2">2</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label>校验</label>
            <select v-model="config.serial.parity" :disabled="connected">
              <option v-for="p in parityOptions" :key="p" :value="p">
                {{ p === "none" ? "无" : p === "odd" ? "奇校验" : "偶校验" }}
              </option>
            </select>
          </div>

          <div class="connect-btns">
            <button v-if="!connected" class="btn btn-primary" @click="connect" :disabled="!config.serial.port">
              连接
            </button>
            <button v-else class="btn btn-danger" @click="disconnect">
              断开
            </button>
          </div>
        </section>

        <section class="panel">
          <h3>显示设置</h3>
          
          <div class="form-group">
            <label class="checkbox">
              <input type="checkbox" v-model="config.display.auto_scroll" />
              <span>自动滚动</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox">
              <input type="checkbox" v-model="config.display.show_timestamp" />
              <span>显示时间戳</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox">
              <input type="checkbox" v-model="config.display.show_hex" />
              <span>显示十六进制</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox">
              <input type="checkbox" v-model="config.display.terminal_mode" />
              <span>终端模式</span>
            </label>
          </div>

          <div class="form-group">
            <label>字体大小</label>
            <input type="range" v-model.number="config.display.font_size" min="10" max="24" />
            <span>{{ config.display.font_size }}px</span>
          </div>
        </section>
      </aside>

      <!-- 主区域 -->
      <main class="content">
        <!-- 搜索栏 -->
        <div v-if="showSearch" class="search-bar">
          <input
            v-model="searchText"
            placeholder="搜索..."
            @keydown.enter="findNext"
            @keydown.escape="showSearch = false"
          />
          <button @click="findPrev">↑</button>
          <button @click="findNext">↓</button>
          <button @click="showSearch = false">✕</button>
          <span v-if="searchText">{{ filteredLog.length }} 条结果</span>
        </div>

        <!-- 终端显示区 -->
        <div
          ref="terminalRef"
          class="terminal"
          :class="{ 'terminal-interactive': config.display.terminal_mode }"
          :style="{ fontSize: config.display.font_size + 'px' }"
        >
          <!-- 传统日志模式 -->
          <template v-if="!config.display.terminal_mode">
            <div
              v-for="(entry, i) in filteredLog"
              :key="i"
              class="log-entry"
              :class="[entry.direction, { highlight: searchText && (entry.data.toLowerCase().includes(searchText.toLowerCase()) || entry.hex.toLowerCase().includes(searchText.toLowerCase())) }]"
            >
              <span v-if="config.display.show_timestamp" class="timestamp">[{{ entry.timestamp }}]</span>
              <span class="direction">{{ entry.direction === "tx" ? "TX" : "RX" }}:</span>
              <span class="data">{{ entry.data }}</span>
              <span v-if="config.display.show_hex" class="hex">| {{ entry.hex }}</span>
            </div>
          </template>
          <!-- xterm 终端容器 -->
          <div 
            v-show="config.display.terminal_mode" 
            ref="xtermContainerRef" 
            class="xterm-container"
          ></div>
          <div v-if="dataLog.length === 0 && !config.display.terminal_mode" class="empty-hint">
            等待数据...
          </div>
          <div v-if="config.display.terminal_mode && !connected" class="empty-hint xterm-hint">
            请先连接串口...
          </div>
        </div>

        <!-- 底部工具栏 -->
        <div class="bottom-toolbar">
          <div class="toolbar-row">
            <div v-if="!config.display.terminal_mode" class="send-settings">
              <label class="checkbox-inline">
                <input type="checkbox" v-model="config.serial.hex_mode" />
                <span>HEX</span>
              </label>
              <label class="checkbox-inline">
                <input type="checkbox" v-model="config.serial.append_newline" />
                <span>换行</span>
              </label>
              <select v-if="config.serial.append_newline" v-model="config.serial.newline_type" class="select-small">
                <option v-for="opt in newlineOptions" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </option>
              </select>
            </div>
            <div class="toolbar-actions">
              <button class="btn btn-small" @click="clearLog">清空</button>
              <button class="btn btn-small" @click="saveLog">保存日志</button>
            </div>
          </div>
          <div v-if="!config.display.terminal_mode" class="send-area">
            <textarea
              v-model="sendText"
              :placeholder="config.serial.hex_mode ? '输入十六进制数据 (如: 48 65 6C 6C 6F)' : '输入要发送的内容...'"
              @keydown.ctrl.enter="send"
              :disabled="!connected"
            ></textarea>
            <button class="btn btn-primary btn-send" @click="send" :disabled="!connected || !sendText">
              发送
            </button>
          </div>
        </div>
      </main>
    </div>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

:root {
  --bg-primary: #1e1e2e;
  --bg-secondary: #2a2a3e;
  --bg-tertiary: #363650;
  --text-primary: #e4e4e7;
  --text-secondary: #a0a0b0;
  --accent: #7c3aed;
  --accent-hover: #8b5cf6;
  --success: #10b981;
  --danger: #ef4444;
  --border: #404050;
  --tx-color: #60a5fa;
  --rx-color: #34d399;
}

body {
  font-family: "Segoe UI", system-ui, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* Toolbar */
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 16px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 16px;
}

.logo .icon {
  font-size: 20px;
}

.status {
  font-size: 14px;
  color: var(--text-secondary);
}

.status.connected {
  color: var(--success);
}

/* Main Content */
.main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* Sidebar */
.sidebar {
  width: 280px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  padding: 12px;
}

.panel {
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 10px;
  margin-bottom: 10px;
}

.panel h3 {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.form-group {
  margin-bottom: 10px;
}

.form-group label {
  display: block;
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.form-group select,
.form-group input[type="text"],
.form-group input[type="number"] {
  width: 100%;
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 13px;
  -webkit-user-select: text;
  user-select: text;
}

.form-group select:disabled,
.form-group input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.baud-select {
  display: flex;
  gap: 4px;
}

.baud-select select {
  flex: 1;
}

.custom-baud-input {
  margin-top: 4px;
  width: 100%;
  padding: 6px 8px;
  background: var(--bg-primary);
  border: 1px solid var(--accent);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 13px;
}

.form-row {
  display: flex;
  gap: 8px;
}

.form-row .form-group {
  flex: 1;
}

.port-select {
  display: flex;
  gap: 4px;
}

.port-select select {
  flex: 1;
}

.btn-icon {
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn-icon:hover {
  background: var(--bg-tertiary);
}

.checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 13px;
}

.checkbox input {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
}

.connect-btns {
  margin-top: 16px;
}

/* Buttons */
.btn {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--accent);
  color: white;
  width: 100%;
}

.btn-primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-danger {
  background: var(--danger);
  color: white;
  width: 100%;
}

.btn-danger:hover:not(:disabled) {
  background: #dc2626;
}

.btn-small {
  padding: 6px 12px;
  font-size: 12px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border: 1px solid var(--border);
}

.btn-small:hover {
  background: var(--border);
}

/* Content Area */
.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 12px;
  overflow: hidden;
}

/* Search Bar */
.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  background: var(--bg-secondary);
  border-radius: 6px;
  margin-bottom: 8px;
}

.search-bar input {
  flex: 1;
  padding: 6px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 13px;
}

.search-bar button {
  padding: 6px 10px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  cursor: pointer;
}

.search-bar button:hover {
  background: var(--border);
}

/* Terminal */
.terminal {
  flex: 1;
  position: relative;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  overflow-y: auto;
  font-family: "Cascadia Code", "Fira Code", Consolas, monospace;
  line-height: 1.6;
}

.log-entry {
  padding: 2px 0;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-entry.tx {
  color: var(--tx-color);
}

.log-entry.rx {
  color: var(--rx-color);
}

.log-entry.highlight {
  background: rgba(124, 58, 237, 0.3);
}

.timestamp {
  color: var(--text-secondary);
  margin-right: 8px;
}

.direction {
  font-weight: 600;
  margin-right: 8px;
}

.hex {
  color: var(--text-secondary);
  font-size: 0.9em;
  margin-left: 8px;
}

.empty-hint {
  color: var(--text-secondary);
  text-align: center;
  padding: 40px;
}

/* Bottom Toolbar */
.bottom-toolbar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 8px;
}

.toolbar-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* Toolbar Actions */
.toolbar-actions {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

/* Send Settings */
.send-settings {
  display: flex;
  align-items: center;
  gap: 12px;
}

.checkbox-inline {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  cursor: pointer;
  color: var(--text-secondary);
}

.checkbox-inline input {
  width: 14px;
  height: 14px;
  accent-color: var(--accent);
}

.checkbox-inline span {
  white-space: nowrap;
}

.select-small {
  padding: 4px 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 12px;
}

/* Send Area */
.send-area {
  display: flex;
  gap: 8px;
}

.send-area textarea {
  flex: 1;
  height: 80px;
  padding: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-family: "Cascadia Code", "Fira Code", Consolas, monospace;
  font-size: 13px;
  resize: none;
}

.send-area textarea:disabled {
  opacity: 0.5;
}

.btn-send {
  width: 80px;
  height: 80px;
}

/* Interactive Terminal Mode */
.terminal-interactive {
  padding: 0 !important;
}

.xterm-container {
  width: 100%;
  height: 100%;
}

.xterm-hint {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 10;
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--bg-primary);
}

::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}
</style>