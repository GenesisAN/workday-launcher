# workday-launcher

仅在中国工作日运行指定程序的命令行工具。

## 快速开始

1. 安装 Rust（stable）。
2. 生成本地配置：
   - 复制 `config.example.toml` 为 `config.toml`，并按需修改 `command/args/workdir/env`。
3. 获取节假日数据（下载到 `data/CN/*.json`）：
   - PowerShell：`powershell -ExecutionPolicy Bypass -File .\scripts\fetch-holidays.ps1 -Years 2025,2026`
4. 运行：
   - `cargo run`

> 说明：本项目默认把配置中的相对路径按“config.toml 所在目录”解析。

## 配置说明（节选）

- `holiday_json_files`：节假日 JSON 文件路径列表（支持多个年份）。
- `missing_policy`：找不到日期时的默认行为：
  - `allow`：仅按周末/工作日规则判断
  - `deny`：找不到就不执行

## 节假日数据来源

- Source (JSON): https://unpkg.com/holiday-calendar@1.3.0/data/CN/
- 本工具只读取你本地的 JSON 文件；请自行遵循上游数据的版权/许可条款。

## 提交到 GitHub 的建议

- `config.toml`：建议不要提交（含本机路径/隐私/环境变量），仓库中提供 `config.example.toml`。
- `data/CN/*.json`：默认已在 `.gitignore` 里忽略；除非你确认上游许可允许再分发，否则建议让使用者用脚本下载。
- `target/`：Rust 构建产物，永远不要提交。
