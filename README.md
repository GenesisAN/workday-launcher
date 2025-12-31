# workday-launcher

**工作日启动器** —— 只在工作日执行指定程序，自动跳过周末和法定节假日。支持调休的工作日。
当然也可以配置成只在节假日执行。

## 快速开始

1. 安装 Rust（stable）。
2. 生成本地配置：
   - 复制 `config.example.toml` 为 `config.toml`，并按需修改 `command/args/workdir/env`。
3. 获取节假日数据（下载到 `data/*.json`）：
   - Git Bash / macOS / Linux（默认 CN）：`bash ./scripts/fetch-holidays.sh --years 2025,2026`
   - 切换地区示例：`bash ./scripts/fetch-holidays.sh --region JP --years 2025,2026`
   - PowerShell（可选）：`powershell -ExecutionPolicy Bypass -File .\scripts\fetch-holidays.ps1 -Years 2025,2026`
4. 运行：
   - `cargo run`

> 说明：本项目默认把配置中的相对路径按“config.toml 所在目录”解析。

## 配置说明

配置文件为 `config.toml`，可从 `config.example.toml` 复制后修改。

> **路径解析规则**：配置中的所有相对路径都以 `config.toml` 所在目录为基准解析。

### 启动程序配置

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|:----:|--------|------|
| `command` | 字符串 | ✅ | - | 要启动的程序 |
| `args` | 字符串数组 | ❌ | `[]` | 传给程序的命令行参数 |
| `workdir` | 字符串 | ❌ | 继承当前目录 | 程序的工作目录 |

**`command` 支持三种写法：**
- 绝对路径：`"C:\\Program Files\\MyApp\\app.exe"`
- 相对路径：`"./bin/app.exe"`（相对于 config.toml 所在目录）
- PATH 命令：`"python"`、`"cmd"` 等系统 PATH 中的命令

### 节假日配置

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|:----:|--------|------|
| `holiday_json_files` | 字符串数组 | ✅ | - | 节假日 JSON 文件路径列表 |
| `missing_policy` | 字符串 | ❌ | `"allow"` | 日期不在 JSON 中时的处理策略 |
| `invert` | 布尔值 | ❌ | `false` | 反转模式：只在节假日/周末执行 |

**`missing_policy` 取值：**

| 值 | 行为（正常模式） | 行为（反转模式） |
|----|------------------|------------------|
| `"allow"` | 周一至周五执行，周末跳过 | 周末执行，周一至周五跳过 |
| `"deny"` | 日期不在 JSON 中时跳过 | 日期不在 JSON 中时执行 |

**节假日类型判定逻辑：**

| JSON 中的 `type` | 正常模式 | 反转模式 |
|------------------|----------|----------|
| `"public_holiday"` | 跳过（法定节假日） | 执行 |
| `"transfer_workday"` | 执行（调休补班） | 跳过 |
| 其他/未知类型 | 回退到周末规则判断 | 回退后反转 |

### 环境变量配置

在 `[env]` 表中设置程序运行时的环境变量：

```toml
[env]
RUST_LOG = "info"
MY_VAR = "value"
```

### 完整配置示例

```toml
# 要启动的程序（必填）
command = "C:\\Program Files\\MyApp\\app.exe"

# 命令行参数（可选）
args = ["--config", "settings.json"]

# 工作目录（可选）
workdir = "C:\\workspace"

# 节假日数据文件（必填，支持多年份）
holiday_json_files = ["./data/2025.json", "./data/2026.json"]

# 缺失日期策略（可选）
missing_policy = "allow"

# 反转模式（可选）：true 表示只在节假日/周末执行
invert = false

# 环境变量（可选）
[env]
RUST_LOG = "info"
```

## 命令行参数

```
workday-launcher [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `--config <PATH>` | 指定配置文件路径（默认自动查找 `config.toml`） |
| `--date <YYYY-MM-DD>` | 覆盖日期，用于测试（默认使用当前日期） |
| `--dry-run` | 演练模式：只打印决策与命令，不实际执行 |
| `--verbose` | 输出详细日志 |
| `--help` | 显示帮助信息 |
| `--version` | 显示版本号 |

**配置文件查找顺序**（未指定 `--config` 时）：
1. 当前工作目录下的 `config.toml`
2. 可执行文件所在目录下的 `config.toml`

## 节假日数据来源

- Source (JSON): https://unpkg.com/holiday-calendar@1.3.0/data/CN/ （把 CN 替换成你的地区代码即可）
- 本工具只读取你本地的 JSON 文件；请自行遵循上游数据的版权/许可条款。