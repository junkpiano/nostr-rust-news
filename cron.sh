#!/usr/bin/env sh
set -eu

# Runs the latest GitHub release binary once. Schedule this script with cron (or any scheduler).

cd "$(dirname "$0")"

if [ -z "${NOSTR_NSEC:-}" ]; then
  echo "NOSTR_NSEC is required" >&2
  exit 1
fi

if [ -z "${NOSTR_RELAYS:-}" ]; then
  echo "NOSTR_RELAYS is required (comma-separated relay URLs)" >&2
  exit 1
fi

REPO="${REPO:-junkpiano/nostr-rust-news}"
APP_NAME="${APP_NAME:-nostr-rust-news}"
ASSET_NAME="${ASSET_NAME:-}"

if [ -z "$ASSET_NAME" ]; then
  echo "ASSET_NAME is required (exact GitHub release asset filename)" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to download releases" >&2
  exit 1
fi

BIN_DIR="${BIN_DIR:-bin}"
mkdir -p "$BIN_DIR"

latest_json="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")"
latest_tag="$(printf '%s\n' "$latest_json" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"

if [ -z "$latest_tag" ]; then
  echo "Failed to determine latest release tag for $REPO" >&2
  exit 1
fi

asset_url="$(
  printf '%s\n' "$latest_json" | awk -v name="$ASSET_NAME" '
    $0 ~ "\"name\": \""name"\"" {want=1}
    want && $0 ~ "\"browser_download_url\"" {
      gsub(/.*"browser_download_url":[[:space:]]*"/, "");
      gsub(/".*/, "");
      print;
      exit
    }'
)"

if [ -z "$asset_url" ]; then
  echo "Failed to find asset \"$ASSET_NAME\" in latest release $latest_tag" >&2
  exit 1
fi

tag_file="$BIN_DIR/.release-tag"
bin_path="$BIN_DIR/$APP_NAME"

if [ ! -x "$bin_path" ] || [ ! -f "$tag_file" ] || [ "$(cat "$tag_file")" != "$latest_tag" ]; then
  tmp_dir="$(mktemp -d)"
  asset_path="$tmp_dir/$ASSET_NAME"

  echo "Downloading $asset_url"
  curl -fsSL -o "$asset_path" "$asset_url"

  case "$ASSET_NAME" in
    *.tar.gz|*.tgz)
      tar -C "$tmp_dir" -xzf "$asset_path"
      ;;
    *.zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "$asset_path" -d "$tmp_dir"
      else
        echo "unzip is required to extract $ASSET_NAME" >&2
        exit 1
      fi
      ;;
    *)
      # Assume it's a raw binary
      mv "$asset_path" "$bin_path"
      chmod +x "$bin_path"
      printf '%s' "$latest_tag" > "$tag_file"
      rm -rf "$tmp_dir"
      exec "$bin_path" "$@"
      ;;
  esac

  if [ ! -f "$tmp_dir/$APP_NAME" ]; then
    # Try to locate the binary in extracted contents
    found="$(find "$tmp_dir" -type f -name "$APP_NAME" -maxdepth 3 | head -n1 || true)"
    if [ -z "$found" ]; then
      echo "Failed to locate binary $APP_NAME in release asset" >&2
      exit 1
    fi
    mv "$found" "$bin_path"
  else
    mv "$tmp_dir/$APP_NAME" "$bin_path"
  fi

  chmod +x "$bin_path"
  printf '%s' "$latest_tag" > "$tag_file"
  rm -rf "$tmp_dir"
fi

exec "$bin_path" "$@"
