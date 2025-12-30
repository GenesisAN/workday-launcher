param(
  [int[]]$Years = @(2025, 2026),
  [string]$Region = "CN"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
$dataDir = Join-Path $repoRoot "data"

New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

foreach ($y in $Years) {
  $url = "https://unpkg.com/holiday-calendar@1.3.0/data/$Region/$y.json"
  $out = Join-Path $dataDir "$y.json"

  Write-Host "Downloading $url"
  Invoke-WebRequest -Uri $url -OutFile $out
  Write-Host "Saved to $out"
}

Write-Host "Done. Update holiday_json_files in config.toml if needed."
