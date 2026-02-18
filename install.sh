#!/usr/bin/env sh
set -eu

REPO="${REPO:-junkpiano/nostr-rust-news}"
APP_NAME="${APP_NAME:-nostr-rust-news}"

if [ "$#" -ne 0 ]; then
  echo "install.sh does not take arguments. It auto-detects your system." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

detect_targets() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "$os" in
    linux) ;;
    darwin) ;;
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

  case "$os" in
    linux)
      # Prefer musl when detected, then try gnu as fallback.
      if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
        printf '%s\n' "${arch_part}-unknown-linux-musl"
      fi
      printf '%s\n' "${arch_part}-unknown-linux-gnu"
      ;;
    darwin)
      printf '%s\n' "${arch_part}-apple-darwin"
      ;;
  esac
}

pick_install_path() {
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

TARGETS="$(detect_targets)"

release_json="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")"
asset_urls="$(printf '%s\n' "$release_json" | sed -n 's/.*"browser_download_url":[[:space:]]*"\([^"]*\)".*/\1/p')"

asset_url=""
selected_target=""
old_ifs="${IFS}"
IFS='
'
for target in $TARGETS; do
  match="$(printf '%s\n' "$asset_urls" | grep "/${APP_NAME}-.*-${target}\\.tar\\.gz$" | head -n1 || true)"
  if [ -n "$match" ]; then
    asset_url="$match"
    selected_target="$target"
    break
  fi
done
IFS="${old_ifs}"

if [ -z "$asset_url" ]; then
  echo "No matching release asset found for detected targets:" >&2
  printf '%s\n' "$TARGETS" >&2
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

echo "Installed $APP_NAME ($selected_target) to $dest_path"
