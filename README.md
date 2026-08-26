# CC-Switch Web（Web 包装版）

> 本仓库是 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) **v3.20.0** 的 Web 包装版。
> 在保留官方桌面能力的同时，通过 `crates/server`（Rust axum HTTP/WS 服务）暴露完整 Web 前端，
> 并附带 **用户名 + 密码** 认证保护敏感 API（API key、provider 切换、MCP 配置等）。

---

## 与官方版的关系

- **上游**：[farion1231/cc-switch](https://github.com/farion1231/cc-switch)（v3.20.0）
- **本仓库定位**：仅在官方 release 之上叠加一层 Web 服务 + 认证，所有业务逻辑（providers、prompts、skills、MCP、settings、SQL 导入导出等）都复用 `crates/core` 与 `src-tauri` headless 模式
- **跟随方式**：参见 `WEB_BUILD.md` §3 — 用 `git cherry-pick` 把本仓库唯一的 web 包装 commit 重新应用到下一个官方 release 分支

## 当前分支

| 分支 | 用途 |
|---|---|
| `main` | 远端默认分支，与 `web-wrapper-v3.20` 同步指向同一份 commit（基线 `0b5da51` + 4 个 web 包装提交）|
| `web-wrapper-v3.20` | 与 `main` 同步；保留以兼容旧 URL 与本地跟踪 |
| `origin/*` | 上游 farion1231 远端引用（仅 `git fetch` 拉取参考，不在本地 checkout）|

## 核心特性

- ✅ **Provider / MCP / Prompt / Skill / Settings 全套**（与桌面版一致）
- ✅ **SQL 导入导出**（浏览器上传/下载）
- ✅ **WebSocket 实时事件**（`/api/ws`）
- ✅ **用户名 + 密码登录**（Cookie session，HttpOnly，7 天，bcrypt cost 12）
- ⚠️ **未覆盖**：v3.20.0 新增的 pi、Grok Build、XAI OAuth、Model pricing 模块（详见 `WEB_BUILD.md` §5）
- ❌ **无系统托盘**（仅桌面端特性）

## 快速开始

> 详细步骤、依赖版本、systemd 单元、生产部署见 [`WEB_BUILD.md`](./WEB_BUILD.md)。

```bash
# 1. 拉取代码（默认分支 main）
git clone https://github.com/JecoShen/cc-switch-web.git
cd cc-switch-web

# 2. 准备配置（必改字段：username + bcrypt password_hash）
cp /root/.cc-switch/web-auth.json /root/.cc-switch/web-auth.json  # 首次部署时创建
chmod 600 /root/.cc-switch/web-auth.json

# 3. 构建
./build-web-release.sh           # 前端 + 后端 release 构建
# 或者仅前端
npm install && npm run build:web

# 4. 启动（systemd 推荐）
sudo tee /etc/systemd/system/cc-switch-web.service > /dev/null <<'UNIT'
# 参考 WEB_BUILD.md §2.2 完整单元文件
UNIT
sudo systemctl enable --now cc-switch-web.service

# 5. 验证
curl -s https://your-domain.example/api/invoke \
     -X POST -H "Content-Type: application/json" \
     -d '{"command":"auth.status","payload":{}}'
# 期望: {"result":{"enabled":true}}
```

## 凭据安全

**本仓库不存任何具体账号或密码。** `WEB_BUILD.md` §2.1 也已脱敏。

凭据仅存于：
1. 部署主机的 `/root/.cc-switch/web-auth.json`（`chmod 600`，bcrypt cost 12）
2. 你的密码管理器

修改密码：参见 `WEB_BUILD.md` §4。

## 升级到新版本

```bash
# 仓库内已提供一键脚本
./upgrade-web.sh
```

脚本会：fetch 远端 → 新建升级分支 → cherry-pick web 包装 commit → 重新构建 → 重启 systemd 单元 → 端到端验证。

## 故障排查

| 症状 | 看哪里 |
|---|---|
| 域名 502 | `systemctl status cc-switch-web.service` + `journalctl -u cc-switch-web.service -n 50` |
| 登录失败 | `python3 -c "import bcrypt; print(bcrypt.verify(b'你的密码', b'你的哈希'))"` 验证 |
| API 401 | 浏览器 DevTools 看 `cc-switch-session` cookie 是否存在 |
| 编译失败 | `rustc --version` ≥ 1.85，`node --version` ≥ 20 |
| WebSocket 断连 | nginx 升级头是否在反代配置中（`proxy_set_header Upgrade $http_upgrade;`） |

完整排查表见 `WEB_BUILD.md` §7。

## 致谢

- [farion1231/cc-switch](https://github.com/farion1231/cc-switch) — 本项目的全部业务代码来源
- [cp-yu](https://github.com/cp-yu) — v3.17.1 的 web 框架原作者
