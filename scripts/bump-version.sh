#!/usr/bin/env bash
set -euo pipefail

# bump-version.sh
#
# 自动修改发版前需要更新的版本字段。
#
# 用法:
#   ./scripts/bump-version.sh 0.2.9                # 使用今天日期
#   ./scripts/bump-version.sh 0.2.9 2026-09-11
#
# 会修改:
#   package.json
#   tauri/src-tauri/Cargo.toml
#   tauri/src-tauri/Cargo.lock
#   tauri/src-tauri/tauri.conf.json
#   snap/snapcraft.yaml
#   io.github.win12_online.win12_desktop.metainfo.xml （只新增 release 条目和日期）

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

NEW_VERSION="${1:-}"
if [[ -z "$NEW_VERSION" ]]; then
  echo "用法: $0 新版本号 [发布日期]" >&2
  echo "示例: $0 0.2.9" >&2
  echo "      $0 0.2.9 2026-09-11" >&2
  exit 1
fi

RELEASE_DATE="${2:-$(date +%F)}"

if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "错误: 版本号应为 主版本.次版本.修订版本 格式，例如 0.2.9，实际为: $NEW_VERSION" >&2
  exit 1
fi

if [[ ! "$RELEASE_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "错误: 发布日期应为 YYYY-MM-DD 格式，例如 2026-09-11，实际为: $RELEASE_DATE" >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "错误: 需要 node 来执行 scripts/bump-version.mjs" >&2
  exit 1
fi

node "$ROOT_DIR/scripts/bump-version.mjs" "$NEW_VERSION" "$RELEASE_DATE"
