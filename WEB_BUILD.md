# CC-Switch Web Build 指南

> 本仓库是 [farion1231/cc-switch v3.20.0](https://github.com/farion1231/cc-switch) 的 **Web 包装版**，
> 在保留官方桌面能力的同时，通过 `crates/server`（Rust axum HTTP/WS 服务）暴露完整 Web 前端，
> 并附带 **用户名 + 密码** 认证保护敏感 API（API key、provider 切换、MCP 配置等）。

---

## 1. 当前架构

```
┌──────────────────┐  HTTPS  ┌──────────────────────┐  HTTP/WS  ┌────────────────────┐
│   用户浏览器     │ ──────▶ │  Nginx 反向代理      │ ────────▶ │  cc-switch-web     │
│  cc-switch.       │         │  cc-switch.neoshen.  │           │  (Rust axum,       │
│  neoshen.         │         │  dpdns.org:443       │           │   127.0.0.1:17666) │
│  dpdns.org        │         │  → 127.0.0.1:17666   │           └─────────┬──────────┘
└──────────────────┘         └──────────────────────┘                     │
                                                                        ▼
                                                            ┌────────────────────┐
                                                            │ ~/.cc-switch/      │
                                                            │  cc-switch.db      │
                                                            │  web-auth.json     │
                                                            │  settings.json     │
                                                            └────────────────────┘
```

- **前端** (`src/`): React 18 + Vite + Radix UI + Tauri 风格 transport 抽象
- **后端** (`crates/server`): Rust 0.85+ · axum 0.7 · tokio · bcrypt 0.15 · 单端口 17666
- **业务核心** (`crates/core` + `src-tauri` headless 模式): 复用 cc-switch v3.20.0 的所有数据/服务层
- **认证**: Cookie session（HttpOnly, 7 天），bcrypt 12 哈希
- **传输**: HTTP `POST /api/invoke` + WebSocket `/api/ws`（事件推送）
- **数据**: 与桌面版完全共享 `~/.cc-switch/`

---

## 2. 部署现状

| 项目 | 值 |
|------|-----|
| 公网域名 | `https://cc-switch.neoshen.dpdns.org` |
| 反向代理 | Nginx，宝塔面板维护（扩展配置：`extension/cc-switch.neoshen.dpdns.org/novnc.conf`） |
| 后端绑定 | `127.0.0.1:17666`（仅本机，外部通过域名 + 反代访问） |
| 数据目录 | `/root/.cc-switch/`（`cc-switch.db`、`settings.json`、`web-auth.json`） |
| 认证配置 | `/root/.cc-switch/web-auth.json`（权限 600，bcrypt cost 12） |
| 进程管理 | **systemd** 单元 `/etc/systemd/system/cc-switch-web.service`（`auto-restart` + 开机自启） |
| 进程用户 | root（systemd 单元的 `User=root`，`ProtectSystem=strict` 配合 `ReadWritePaths` 仅允许写 `/root/.cc-switch` 和 `/var/log`） |

### 2.1 当前生效的账号/密码

> ⚠️ **生产环境强凭证**（每次部署/迁移时务必同步更新本节）

- **用户名**: `qu9z298xjt4y`（12 位小写字母 + 数字，bcrypt 哈希存储）
- **密码**: `.+%=B(pmP[d]#f2)=<$d`（20 位，9 类特殊符号 + 大小写 + 数字）
- **配置文件**: `/root/.cc-switch/web-auth.json`（`chmod 600`）
- **会话有效期**: 7 天（cookie `cc-switch-session`，`HttpOnly; SameSite=Strict; Max-Age=604800; Path=/`）
- **失败响应**: `{"error":"Unauthorized"}`（401）/ `{"success":false,"error":"Invalid username or password"}`

> 密码原始值**只存于本节**作为部署留底；hash 之外的明文无第二份。如需修改，参见 §4.3。

### 2.2 进程管理（systemd）

```bash
# 状态
systemctl status cc-switch-web.service

# 重启（不影响远端连接，因为 systemd 立即拉起新 PID）
systemctl restart cc-switch-web.service

# 实时日志
journalctl -u cc-switch-web.service -f         # systemd journal
tail -f /var/log/cc-switch-web.log            # 业务日志（标准输出重定向）

# 停服
systemctl stop cc-switch-web.service

# 取消开机自启
systemctl disable cc-switch-web.service
```

**安全加固**（已配置于单元文件）：

- `Restart=always` + `RestartSec=5` — 进程挂掉后 5 秒自动拉起
- `NoNewPrivileges` / `PrivateTmp` / `ProtectSystem=strict` — 进程无法越权写系统目录
- `ReadWritePaths=/root/.cc-switch /var/log` — 仅这两处可写
- `ProtectKernelTunables` / `ProtectControlGroups` / `LockPersonality` — 内核态保护
- `StandardOutput=append:/var/log/cc-switch-web.log` — 日志落盘

---

## 3. 跟随后续官方更新

> **核心目标**：不破坏与 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 官方的同步关系。

### 3.1 当前 Git 状态

- **远端 (origin)**: `https://github.com/farion1231/cc-switch.git`
- **当前分支**: `web-wrapper-v3.20`
- **基线提交**: `0b5da51` (v3.20.0 release notes)
- **Web 包装层**: 1 个 commit (`d89eaf5`) — 全部 web 改造 + 认证逻辑

### 3.2 升级到新版本的标准流程

```bash
# 1. 拉取官方最新代码（fetch main 分支的所有 tag 和分支）
cd /www/wwwroot/cc-switch.neoshen.dpdns.org
git fetch origin --tags
git fetch origin main

# 2. 切换到一个新分支（避免污染 web-wrapper-v3.20）
git checkout -b web-wrapper-v3.21 origin/main

# 3. 重新应用 web 包装补丁
#    方案 A（推荐）: 合并上一版的 web 包装 commit
git cherry-pick d89eaf5
#    解决可能出现的 src-tauri/Cargo.toml / src/ 冲突

# 4. 重新安装前端依赖（package.json 可能变了）
npm install

# 5. 重新构建前端 + 后端
npm run build:web
cargo build --release --manifest-path crates/server/Cargo.toml

# 6. 重启服务（systemd 会自动用新二进制）
systemctl restart cc-switch-web.service
sleep 2
systemctl status cc-switch-web.service --no-pager

# 7. 验证
curl -s https://cc-switch.neoshen.dpdns.org/api/invoke \
     -X POST -H "Content-Type: application/json" \
     -d '{"command":"auth.status","payload":{}}'
# 期望: {"result":{"enabled":true}}
```

### 3.3 冲突可能的位置

升级时 `git cherry-pick d89eaf5` 最可能在以下位置冲突：

| 文件 | 冲突原因 | 解决思路 |
|------|---------|---------|
| `src-tauri/Cargo.toml` | 官方新增/删除依赖 | 保留官方版本，重做 headless feature 包装（把所有新加的非 Tauri 依赖保持 `optional = false`，把所有 Tauri 插件设 `optional = true` 并加入 `desktop` feature 列表） |
| `src-tauri/src/lib.rs` | 官方新增模块 / 修改 run() | 重新审查 `#[cfg(feature = "desktop")]` 包裹范围；新增的 `mod xxx;` 全部不需要 feature gate（headless 也能用） |
| `src/main.tsx` / `src/App.tsx` | 官方新增路由 / 主题 | 保留新增的路由/主题，但确保 `bootstrap.web.ts` 与之兼容 |
| `vite.config.ts` | 官方改了 vite 配置 | 以新官方配置为基础，把 web 模式的 `@platform/*` alias 加回去 |
| `package.json` | 官方新增 npm 依赖 | 保留新增的依赖，把 web 模式的 scripts (`build:web` 等) 加回去 |

### 3.4 一键升级脚本（参考）

把下面的内容存为 `upgrade-web.sh`：

```bash
#!/bin/bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_ROOT"

WEB_COMMIT=$(git log --oneline --grep "web wrapper" -n 1 | awk '{print $1}')

echo "▶ 拉取远端最新代码..."
git fetch origin --tags
git fetch origin main

echo "▶ 创建升级分支..."
git checkout -b "web-wrapper-$(date +%Y%m%d)" origin/main

echo "▶ 应用 web 包装补丁..."
if git cherry-pick "$WEB_COMMIT"; then
  echo "✓ cherry-pick 成功"
else
  echo "✗ 出现冲突，请手动解决后执行 git cherry-pick --continue"
  exit 1
fi

echo "▶ 安装依赖..."
npm install

echo "▶ 构建..."
npm run build:web
cargo build --release --manifest-path crates/server/Cargo.toml

echo "▶ 重启服务..."
systemctl restart cc-switch-web.service
sleep 2
systemctl status cc-switch-web.service --no-pager

echo "▶ 验证..."
if curl -s "https://cc-switch.neoshen.dpdns.org/api/invoke" \
     -X POST -H "Content-Type: application/json" \
     -d '{"command":"auth.status","payload":{}}' | grep -q '"enabled":true'; then
  echo "✓ 升级成功！访问 https://cc-switch.neoshen.dpdns.org 测试登录"
else
  echo "✗ 服务验证失败，请查看 /var/log/cc-switch-web.log"
  exit 1
fi
```

---

## 4. 账号密码管理

### 4.1 配置文件位置

```
/root/.cc-switch/web-auth.json   (权限 600)
```

### 4.2 配置格式

```json
{
  "username": "your_admin_name",
  "password_hash": "$2b$12$xxxxxx...bcrypt哈希"
}
```

- `username` 字段：可省略，省略时默认 `admin`
- `password_hash` 字段：bcrypt 哈希（cost ≥ 12）

### 4.3 修改密码

```bash
# 1. 生成新密码的 bcrypt 哈希
NEW_HASH=$(python3 -c "import bcrypt; print(bcrypt.hashpw(b'你的新密码', bcrypt.gensalt(12)).decode())")

# 2. 备份并修改配置
cp /root/.cc-switch/web-auth.json /root/.cc-switch/web-auth.json.bak
cat > /root/.cc-switch/web-auth.json <<EOF
{
  "username": "admin",
  "password_hash": "$NEW_HASH"
}
EOF
chmod 600 /root/.cc-switch/web-auth.json

# 3. 重启服务使新密码生效
systemctl restart cc-switch-web.service
```

### 4.4 同时修改账号名 + 密码

```bash
NEW_USER="my_new_user"
NEW_PASS="my_new_pass_!@#\$%^"

# 1. 生成 bcrypt 哈希
NEW_HASH=$(python3 -c "import bcrypt; print(bcrypt.hashpw(b'$NEW_PASS', bcrypt.gensalt(12)).decode())")

# 2. 写新配置
cat > /root/.cc-switch/web-auth.json <<EOF
{
  "username": "$NEW_USER",
  "password_hash": "$NEW_HASH"
}
EOF
chmod 600 /root/.cc-switch/web-auth.json

# 3. 重启
systemctl restart cc-switch-web.service
```

### 4.5 临时禁用认证（仅调试用）

```bash
mv /root/.cc-switch/web-auth.json /root/.cc-switch/web-auth.json.disabled
systemctl restart cc-switch-web.service
```

**警告**：禁用认证后任何人可访问 API！生产环境务必重新启用。

---

## 5. 已知的 v3.20.0 特性未覆盖

由于 web 包装层基于 cp-yu 的 v3.17.1 web 框架，`src-tauri/` 的部分 v3.20.0 新模块**未包含在 web 模式**中：

| 特性 | 桌面版支持 | Web 版支持 | 备注 |
|------|----------|-----------|------|
| Provider CRUD (Claude/Codex/Gemini/Hermes/OpenClaw) | ✅ | ✅ | |
| MCP 服务器管理 | ✅ | ✅ | |
| Prompt 管理 | ✅ | ✅ | |
| Skill 管理 | ✅ | ✅ | |
| Settings / 导入导出 (SQL) | ✅ | ✅ | Web 版通过浏览器上传/下载 |
| WebSocket 实时事件 | ✅ | ✅ | `wss://cc-switch.neoshen.dpdns.org/api/ws` |
| 用户名+密码登录 | ✅ | ✅ | 浏览器 Cookie session |
| **pi** provider 体系 | ✅ | ❌ | v3.20.0 新增 |
| **Grok Build** provider | ✅ | ❌ | v3.20.0 新增 |
| **XAI OAuth** | ✅ | ❌ | v3.20.0 新增 |
| **Model pricing** 模块 | ✅ | ❌ | v3.20.0 新增 |
| **Pi/Grokbuild session usage** | ✅ | ❌ | v3.20.0 新增 |
| **OpenClaw 健康检查** | ✅ | ⚠️ 部分 | |
| 系统托盘 | ✅ | ❌ | 仅桌面端有 |

如需在 Web 版启用上述被标注 ❌ 的特性，需要把对应的 v3.20.0 src-tauri/src/ 新文件
（`pi_config/`、`grok_config.rs`、`xai_oauth_auth.rs`、`model_pricing.rs` 等）合并到
本仓库的 src-tauri 中，并 wire-up 到 `commands/mod.rs` 与 `dispatch.rs`。
这是一次较大的合并工作，建议按需分批进行。

---

## 6. 关键脚本与日志

| 文件 | 用途 |
|------|------|
| `/etc/systemd/system/cc-switch-web.service` | systemd 单元（守护 + 开机自启） |
| `start-web.sh` / `stop-web.sh` | 旧版启停脚本（已弃用，保留以兼容） |
| `build-web-release.sh` | 完整构建脚本（npm + cargo release） |
| `/var/log/cc-switch-web.log` | 服务运行日志（通过 systemd 落盘） |
| `journalctl -u cc-switch-web.service` | systemd 统一日志（推荐查问题用） |
| `crates/server/target/release/cc-switch-web` | 编译产物（二进制） |
| `dist/` | 前端构建产物（被嵌入到 Rust 二进制中） |

---

## 7. 故障排查

| 症状 | 检查点 | 修复 |
|------|-------|------|
| 域名访问 502 | `systemctl status cc-switch-web` | 进程挂了；systemd 会在 5s 内自动拉起，否则 `systemctl start cc-switch-web` |
| 域名访问 404 | nginx conf | 检查 `/www/server/panel/vhost/nginx/cc-switch.neoshen.dpdns.org.conf` 与 `extension/cc-switch.neoshen.dpdns.org/novnc.conf` 是否还在 |
| 登录失败 | bcrypt 哈希 | 用 `python3 -c "import bcrypt; print(bcrypt.verify(b'你的密码', b'你的哈希'))"` 验证 |
| API 返回 `Unauthorized` | Cookie | 重新登录；浏览器 DevTools 看 `cc-switch-session` 是否存在 |
| WebSocket 断连 | nginx 升级头 | 确认 `proxy_set_header Upgrade $http_upgrade;` 在 `novnc.conf` 中 |
| `error: failed to initialize cc-switch core context` | 数据目录权限 | `chmod 700 /root/.cc-switch && chown -R root:root /root/.cc-switch` |
| 编译失败 | Rust 版本 | `rustc --version` 必须 ≥ 1.85（src-tauri/Cargo.toml 要求） |
| 前端白屏 | 浏览器控制台 | 看 Network tab，`/assets/index-*.js` 是否 200；看 Console 错误信息 |
| Systemd 启动失败 | journal | `journalctl -u cc-switch-web.service -n 50 --no-pager` |

---

## 8. 升级路线图

- **v3.21.0**（下一个官方 release）: 重复第 3 节的标准流程
- 优先合并 v3.20.0 缺失特性：pi、Grok Build、XAI OAuth、Model pricing
- 改进 `start-web.sh` 让它能检测 v3.20.0 后续发布并自动适配
