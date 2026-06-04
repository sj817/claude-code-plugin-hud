# claude-code-plugin-hud

给 [Claude Code](https://code.claude.com/docs/zh-CN/statusline) 用的清爽两行状态栏 —— 模型、上下文额度、花费、git、速率限制,一眼可见,又不碍事。

[English](./README.md) · 简体中文

![demo](./demo.png)

---

## 快速开始

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/claude-code-plugin-hud:setup
```

就这样。`:setup` 会自动选好对应你系统的预编译二进制,并把状态栏写进你的配置。下一条消息就能看到 HUD。

> 插件命令带命名空间前缀,所以是 `claude-code-plugin-hud:`。

---

## 这一行行都是啥

```
██░░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 396k/397k(99%) | 5h(25% 57m) · 7d(38% 2d10h) | 📁 D:/Github/claude-code-plugin-hud
```

**第一行 —— 当下状态**

| 段                  | 含义                                       |
| ------------------ | ---------------------------------------- |
| `██░░ 397k/1M(40%)` | 上下文窗口 已用 / 总量(唯一的进度条,按占用变色)        |
| `+2318 -922`       | 本会话新增 / 删除行数                            |
| `💰 $27.62`         | 本会话花费                                    |
| `⏱ 3h4m`           | 自会话开始的挂钟时间                              |
| `🌿 main*`          | git 分支(`*` 表示有未提交改动)                   |
| `v2.1.161 ⚡high(Opus 4.8)` | Claude Code 版本 · 推理强度 · 模型(钉在右上角) |

**第二行 —— 会话与额度**

| 段                       | 含义                                  |
| ----------------------- | ----------------------------------- |
| `🎉Cache: 396k/397k(99%)` | prompt 缓存命中率(高=好,显示绿色)        |
| `5h(25% 57m)`           | 5 小时额度已用,带重置倒计时               |
| `7d(38% 2d10h)`         | 7 天额度已用,带重置倒计时                |
| `📁 …`                   | 工作目录(过长时智能裁剪)                 |

Claude Code 暂时给不了的数据(比如首条消息前的速率限制)会直接省略,布局不变。

---

## 你可能会喜欢的点

- **永远两行。** 不膨胀、不跳动,不会把你的输入框挤跑。
- **适配终端宽度。** 读取 `$COLUMNS`,放不下时优先丢次要段,而不是换行。
- **只有一个进度条。** 只有上下文窗口用进度条,其余都是简短文字。
- **智能路径。** 路径放得下就显示全路径,放不下从顶层裁剪。
- **温和配色。** 标准 ANSI,跟随你的终端主题 —— 上下文 绿→黄→红 越满越警示;缓存命中率健康时为绿。
- **小而快。** 单个约 310 KB 的 Rust 二进制,无运行时依赖。

---

## 配置

可选 —— 开箱即用。

| 环境变量                 | 作用                              |
| -------------------- | ------------------------------- |
| `CLAUDE_HUD_ONELINE` | `1` 只渲染一行(给底部模式行留空间)。     |

`$COLUMNS` / `$LINES` 由 Claude Code 提供(v2.1.153+);终端很矮时 HUD 会自动收缩成一行。

---

## 手动配置

不想用 `:setup` 的话,直接把状态栏指向二进制:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/绝对路径/claude-hud",   // Windows 上用正斜杠
    "padding": 2
  }
}
```

若 Claude Code 版本早于 v2.1.153(不导出 `$COLUMNS`),包一层从 tty 取宽度:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /绝对路径/claude-hud"
```

---

## 从源码编译

```bash
cargo build --release        # -> target/release/claude-hud
```

用模拟输入试跑:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

交叉编译全部平台到 `dist/<triple>/`:

```bash
scripts/build-all.sh
```

---

## 发版

推一个版本 tag,CI 全包 —— 编译 Windows / macOS(x64 + arm64)/ Linux(x64 + arm64),把二进制提交进 `dist/`,并发布 GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

---

## 许可证

[MIT](./LICENSE)
