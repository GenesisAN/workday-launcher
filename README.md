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

## 配置说明（节选）

- `holiday_json_files`：节假日 JSON 文件路径列表（支持多个年份）。
- `missing_policy`：找不到日期时的默认行为：
  - `allow`：仅按周末/工作日规则判断
  - `deny`：找不到就不执行

## 节假日数据来源

- Source (JSON): https://unpkg.com/holiday-calendar@1.3.0/data/CN/ （把 CN 替换成你的地区代码即可）
- 本工具只读取你本地的 JSON 文件；请自行遵循上游数据的版权/许可条款。