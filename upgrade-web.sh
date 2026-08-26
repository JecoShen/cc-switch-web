#!/bin/bash
#
# CC-Switch Web 升级脚本
# 从 farion1231/cc-switch 拉取最新 main，cherry-pick 本仓库的 web 包装补丁，
# 重新构建并重启 systemd 单元。
#
# 用法:
#   ./upgrade-web.sh                            # 默认升级到上游 main 最新
#   DOMAIN=https://my.example ./upgrade-web.sh  # 指定健康检查域名
#   UPSTREAM_REMOTE=origin ./upgrade-web.sh     # 指定上游远端名
#   WEB_COMMIT=d89eaf53 ./upgrade-web.sh        # 显式指定要 cherry-pick 的 web 包装 commit
#
# 前提:
#   1. 当前仓库已 git fetch 过上游（默认远端名 origin 指向 farion1231/cc-switch）
#   2. 已配置 systemd 单元 cc-switch-web.service
#   3. 已安装 node ≥ 20、rustc ≥ 1.85

set -euo pipefail

SCRIPT_SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SCRIPT_SOURCE" ]; do
    SCRIPT_DIR="$(cd -P "$(dirname "$SCRIPT_SOURCE")" && pwd)"
    SCRIPT_SOURCE="$(readlink "$SCRIPT_SOURCE")"
    [[ "$SCRIPT_SOURCE" != /* ]] && SCRIPT_SOURCE="$SCRIPT_DIR/$SCRIPT_SOURCE"
done
PROJECT_ROOT="$(cd -P "$(dirname "$SCRIPT_SOURCE")" && pwd)"
cd "$PROJECT_ROOT"

UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-origin}"
UPSTREAM_BRANCH="${UPSTREAM_BRANCH:-main}"
SERVICE_NAME="${SERVICE_NAME:-cc-switch-web.service}"
DOMAIN="${DOMAIN:-https://cc-switch.neoshen.dpdns.org}"

require_command() {
    local cmd="$1"
    local message="${2:-缺少依赖 $cmd}"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "✗ $message" >&2
        exit 1
    fi
}

require_command git
require_command npm
require_command cargo
require_command systemctl
require_command curl

# 1. 定位要 cherry-pick 的 web 包装 commit
if [ -n "${WEB_COMMIT:-}" ]; then
    echo "▶ 使用环境变量指定的 web 包装 commit: $WEB_COMMIT"
else
    WEB_COMMIT="$(git log --oneline --grep 'web wrapper' -n 1 | awk '{print $1}')"
    if [ -z "$WEB_COMMIT" ]; then
        echo "✗ 找不到 web 包装 commit（git log --grep 'web wrapper' 无结果），请用 WEB_COMMIT=<sha> 显式指定" >&2
        exit 1
    fi
    echo "▶ 自动定位到 web 包装 commit: $WEB_COMMIT"
fi

# 2. 拉取上游
echo "▶ 拉取 $UPSTREAM_REMOTE 最新代码 (tags + $UPSTREAM_BRANCH)..."
git fetch "$UPSTREAM_REMOTE" --tags
git fetch "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"

# 3. 新建升级分支（保留 web-wrapper-v3.20 不被污染）
NEW_BRANCH="web-wrapper-$(date +%Y%m%d-%H%M%S)"
echo "▶ 创建升级分支: $NEW_BRANCH (基于 $UPSTREAM_REMOTE/$UPSTREAM_BRANCH)"
git checkout -b "$NEW_BRANCH" "$UPSTREAM_REMOTE/$UPSTREAM_BRANCH"

# 4. 应用 web 包装补丁
echo "▶ cherry-pick $WEB_COMMIT ..."
if ! git cherry-pick "$WEB_COMMIT"; then
    echo "✗ cherry-pick 出现冲突，请手动解决后执行 'git cherry-pick --continue'，然后再跑一次本脚本" >&2
    exit 1
fi

# 5. 安装前端依赖
echo "▶ npm install ..."
npm install

# 6. 重新构建（前端 + 后端 release）
echo "▶ 构建前端 (npm run build:web)..."
npm run build:web
echo "▶ 构建后端 (cargo build --release)..."
cargo build --release --manifest-path crates/server/Cargo.toml

# 7. 重启 systemd 单元
echo "▶ 重启 $SERVICE_NAME ..."
sudo systemctl restart "$SERVICE_NAME"
sleep 2
if ! sudo systemctl is-active --quiet "$SERVICE_NAME"; then
    echo "✗ $SERVICE_NAME 未处于 active 状态，请查看 'journalctl -u $SERVICE_NAME -n 50'" >&2
    exit 1
fi
sudo systemctl status "$SERVICE_NAME" --no-pager | head -10

# 8. 端到端验证
echo "▶ 验证 $DOMAIN ..."
HEALTH_URL="$DOMAIN/api/invoke"
RESPONSE="$(curl -fsS "$HEALTH_URL" \
    -X POST -H 'Content-Type: application/json' \
    -d '{"command":"auth.status","payload":{}}' || true)"
if echo "$RESPONSE" | grep -q '"enabled":true'; then
    echo "✓ 升级成功！访问 $DOMAIN 测试登录"
else
    echo "✗ 服务验证失败，响应: $RESPONSE" >&2
    echo "  请查看 /var/log/cc-switch-web.log 或 'journalctl -u $SERVICE_NAME -n 50'" >&2
    exit 1
fi
