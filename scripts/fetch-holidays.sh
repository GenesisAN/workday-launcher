#!/usr/bin/env bash
set -euo pipefail

# Cross-platform (Git Bash / macOS / Linux) holiday data fetcher.
# Data source: https://unpkg.com/holiday-calendar@1.3.0/data/<REGION>/<YEAR>.json

REGION="CN"
YEARS=(2025 2026)
PACKAGE_VERSION="1.3.0"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/fetch-holidays.sh [--region CN] [--years 2025,2026]
  ./scripts/fetch-holidays.sh -r CN 2025 2026

Options:
  -r, --region   Region code (default: CN)
  -y, --years    Comma-separated years (default: 2025,2026)
  -h, --help     Show this help

Examples:
  ./scripts/fetch-holidays.sh
  ./scripts/fetch-holidays.sh --region JP --years 2025,2026
  ./scripts/fetch-holidays.sh -r CN 2025 2026
EOF
}

die() {
  echo "[fetch-holidays] error: $*" >&2
  exit 1
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

is_msys_windows() {
  case "$(uname -s 2>/dev/null || echo unknown)" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

download() {
  local url="$1"
  local out="$2"

  if have_cmd curl; then
    # Windows Git Bash often uses Schannel; revocation checks can fail offline.
    # Try a couple of variants, then fall back to wget if available.
    if is_msys_windows; then
      if curl -fsSL --ssl-no-revoke "$url" -o "$out"; then
        return 0
      fi
    fi

    if curl -fsSL "$url" -o "$out"; then
      return 0
    fi
  fi

  if have_cmd wget; then
    if wget -q "$url" -O "$out"; then
      return 0
    fi
  fi

  die "download failed (need working curl or wget)"
}

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    -r|--region)
      [[ $# -ge 2 ]] || die "--region requires a value"
      REGION="$2"
      shift 2
      ;;
    -y|--years)
      [[ $# -ge 2 ]] || die "--years requires a value"
      IFS=',' read -r -a YEARS <<< "$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      die "unknown option: $1 (use --help)"
      ;;
    *)
      # Positional years override defaults
      YEARS=("$@")
      break
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$REPO_ROOT/data"

mkdir -p "$DATA_DIR"

for y in "${YEARS[@]}"; do
  [[ "$y" =~ ^[0-9]{4}$ ]] || die "invalid year: $y"

  url="https://unpkg.com/holiday-calendar@${PACKAGE_VERSION}/data/${REGION}/${y}.json"
  out="$DATA_DIR/${y}.json"
  tmp="$out.tmp"

  echo "[fetch-holidays] downloading $url"
  download "$url" "$tmp"
  mv -f "$tmp" "$out"
  echo "[fetch-holidays] saved to $out"
done

echo "[fetch-holidays] done. Update holiday_json_files in config.toml if needed."
