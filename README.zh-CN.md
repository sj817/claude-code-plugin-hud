# claude-code-plugin-hud

用 Rust 编写的 [Claude Code](https://code.claude.com/docs/zh-CN/statusline) 两行状态栏,显示模型、上下文窗口、花费、git 分支和速率限制。

[English](./README.md) · 简体中文

![demo](./demo.png)

## 安装

### 插件

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/reload-plugins
/claude-code-plugin-hud:setup
```

`/reload-plugins` 用来激活刚装好的命令(或者重启 Claude Code)。随后 `/claude-code-plugin-hud:setup` 会挑选对应你系统的预编译二进制,并把 `statusLine` 条目写进你的 `settings.json`。下一条消息就会显示 HUD。

插件无法直接设置 `statusLine`,所以交给命令来做。插件命令带有命名空间,因此有 `claude-code-plugin-hud:` 前缀。

### 脚本

无需插件。它只下载对应你平台的二进制,并替你写好 `statusLine`(已有的 `settings.json` 会先备份):

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/sj817/claude-code-plugin-hud/main/scripts/install.sh | bash
```

```powershell
# Windows
irm https://raw.githubusercontent.com/sj817/claude-code-plugin-hud/main/scripts/install.ps1 | iex
```

安装到 `~/.claude/statusline/`,随时重跑即可更新。日志会跟随你的系统语言(中文或英文)。可选覆盖:`CLAUDE_HUD_VERSION` 固定某个版本(如 `v0.1.6`),`CLAUDE_HUD_LANG` 强制语言(`zh` / `en`)。

## 组成

```text
███░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.161 ⚡high(Opus 4.8)
Quota: 5h 25% 57m · 7d 38% 2d10h | 🎉Cache: 396k/397k(99%) | 📁 D:/Github/claude-code-plugin-hud
```

### 第一行

| 段 | 示例 | 含义 |
| --- | --- | --- |
| 上下文条 | `███░░░░░ 397k/1M(40%)` | 上下文窗口 已用 / 总量。唯一的进度条。低于 70% 绿色,低于 90% 黄色,90% 及以上红色。 |
| 行数变更 | `+2318 -922` | 本会话新增 / 删除的行数。 |
| 花费 | `💰 $27.62` | 本会话花费(美元)。 |
| 时长 | `⏱ 3h4m` | 自会话开始的挂钟时间。 |
| 分支 | `🌿 main*` | git 分支。末尾的 `*` 表示工作区有未提交改动。 |
| 版本 · 强度(模型) | `v2.1.161 ⚡high(Opus 4.8)` | 右对齐:Claude Code 版本、推理强度、模型。 |

### 第二行

| 段 | 示例 | 含义 |
| --- | --- | --- |
| 速率限制 | `Quota: 5h 25% 57m · 7d 38% 2d10h` | 各时间窗的套餐额度用量,带重置倒计时。第二行的焦点:百分比按每档 25% 分四段着色 —— 低于 25% 绿色,低于 50% 黄色,低于 75% 橙色,75% 及以上红色。 |
| 缓存 | `🎉Cache: 396k/397k(99%)` | prompt 缓存 读取 / 总量,带命中率。 |
| 目录 | `📁 D:/Github/...` | 工作目录,过长时从前面裁剪(但不会短于最后一段)。 |

说明:

- 高度始终两行。段不换行;当某一行超出 `$COLUMNS` 时,优先级最低的段会被丢弃。
- 缺失的数据直接省略,不会占位。首次 API 响应前没有速率限制;`/compact` 之后缓存明细为空。
- 配色基于标准 16 色 ANSI,外加粉色 `Cache:` 标签、柔金色目录,以及额度刻度中的橙色档位。进度条和缓存的 `(%)` 保持无色。
- 不显示权限模式(auto/plan):状态栏 JSON 里没有这个字段。

## 配置

唯一的设置是 `CLAUDE_HUD_ONELINE`。设为 `1`(或 `true`)只渲染第一行,从而让 Claude Code 输入框下方的模式行保持可见:

```text
CLAUDE_HUD_ONELINE=1
```

当 `$LINES` 小于 10 时,HUD 也会收成一行。`$COLUMNS` 和 `$LINES` 由 Claude Code v2.1.153+ 提供;缺失时宽度回退为 80。

## 手动配置

想跳过 `:setup`,就把 `statusLine` 直接指向二进制:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-hud",
    "padding": 2
  }
}
```

Windows 上路径用正斜杠书写。若 Claude Code 版本早于 v2.1.153(没有 `$COLUMNS`),用 `stty` 垫片从 tty 读取宽度:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /absolute/path/to/claude-hud"
```

## 从源码编译

```bash
cargo build --release        # -> target/release/claude-hud
```

用模拟输入测试:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

把全部已发布目标交叉编译到 `dist/<triple>/`:

```bash
scripts/build-all.sh
```

二进制很小(因平台而异,大约 290 KB 到 545 KB),且无运行时依赖。已发布目标:Windows x64 和 arm64、macOS x64 和 arm64、Linux glibc x64 和 arm64、Linux musl x64 和 arm64。

## 发版

推一个 `v*` tag。CI 会编译每个目标,把二进制提交进 `dist/`,并发布 GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## 许可证

[MIT](./LICENSE)
