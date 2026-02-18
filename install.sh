#!/usr/bin/env sh
set -eu

REPO="${REPO:-junkpiano/nostr-rust-news}"
APP_NAME="${APP_NAME:-nostr-rust-news}"

usage() {
  cat <<EOF
Usage: $0 [--bin-path PATH] [--install-dir DIR] [--target TARGET]

Installs the latest GitHub release binary for this machine.

Options:
  --bin-path PATH     Install to this exact file path
  --install-dir DIR   Install to DIR/\$APP_NAME
  --target TARGET     Override detected target (e.g. x86_64-unknown-linux-gnu)

Environment:
  REPO                GitHub repo in owner/name format (default: $REPO)
  APP_NAME            Binary name (default: $APP_NAME)
EOF
}

BIN_PATH=""
INSTALL_DIR=""
TARGET=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --bin-path)
      BIN_PATH="${2:-}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:-}"
      shift 2
      ;;
    --target)
      TARGET="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

detect_target() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "$os" in
    linux) os_part="unknown-linux-gnu" ;;
    *)
      echo "Unsupported OS: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    aarch64|arm64) arch_part="aarch64" ;;
    *)
      echo "Unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  printf '%s-%s' "$arch_part" "$os_part"
}

pick_install_path() {
  if [ -n "$BIN_PATH" ]; then
    printf '%s' "$BIN_PATH"
    return
  fi

  if [ -n "$INSTALL_DIR" ]; then
    printf '%s/%s' "$INSTALL_DIR" "$APP_NAME"
    return
  fi

  old_ifs="${IFS}"
  IFS=":"
  for dir in $PATH; do
    if [ -d "$dir" ] && [ -w "$dir" ]; then
      IFS="${old_ifs}"
      printf '%s/%s' "$dir" "$APP_NAME"
      return
    fi
  done
  IFS="${old_ifs}"

  fallback="${HOME}/.local/bin"
  mkdir -p "$fallback"
  printf '%s/%s' "$fallback" "$APP_NAME"
}

if [ -z "$TARGET" ]; then
  TARGET="$(detect_target)"
fi

release_json="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")"
asset_url="$(
  printf '%s\n' "$release_json" \
    | sed -n 's/.*"browser_download_url":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | grep "/${APP_NAME}-.*-${TARGET}\\.tar\\.gz$" \
    | head -n1
)"

if [ -z "$asset_url" ]; then
  echo "No matching release asset found for target: $TARGET" >&2
  exit 1
fi

dest_path="$(pick_install_path)"
dest_dir="$(dirname "$dest_path")"
mkdir -p "$dest_dir"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

archive_path="$tmp_dir/release.tar.gz"
curl -fsSL -o "$archive_path" "$asset_url"
tar -C "$tmp_dir" -xzf "$archive_path"

if [ -f "$tmp_dir/$APP_NAME" ]; then
  found="$tmp_dir/$APP_NAME"
else
  found="$(find "$tmp_dir" -maxdepth 3 -type f -name "$APP_NAME" | head -n1 || true)"
fi

if [ -z "${found:-}" ] || [ ! -f "$found" ]; then
  echo "Failed to locate $APP_NAME inside archive" >&2
  exit 1
fi

cp "$found" "$dest_path"
chmod 0755 "$dest_path"

echo "Installed $APP_NAME to $dest_path"
