# claude-code-plugin-hud

给 [Claude Code](https://code.claude.com/docs/zh-CN/statusline) 用的清爽两行状态栏 —— 模型、上下文额度、花费、git、速率限制,一眼可见,又不碍事。

[English](./README.md) · 简体中文

![demo](./demo.png)

## 安装

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/reload-plugins
/claude-code-plugin-hud:setup
```

`/reload-plugins` 用来激活刚装好的命令(或者直接重启 Claude Code)。然后 `/claude-code-plugin-hud:setup` 会自动选好对应你系统的预编译二进制,并把状态栏写进配置 —— 下一条消息就能看到 HUD。

> 插件命令带插件名作命名空间前缀,所以是 `claude-code-plugin-hud:`。

## 看懂这两行

```
██░░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 396k/397k(99%) | 5h(25% 57m) · 7d(38% 2d10h) | 📁 D:/Github/claude-code-plugin-hud
```

**第一行 —— 当下状态**,从左到右:

- `██░░ 397k/1M(40%)` —— 上下文窗口,已用 / 总量。唯一的进度条,越满越从 绿 → 黄 → 红。
- `+2318 -922` —— 本会话新增 / 删除的行数。
- `💰 $27.62` —— 本会话花费。
- `⏱ 3h4m` —— 自会话开始的挂钟时间。
- `🌿 main*` —— git 分支;`*` 表示有未提交改动。
- `v2.1.161 ⚡high(Opus 4.8)` —— Claude Code 版本、推理强度、模型,钉在右侧。

**第二行 —— 会话与额度:**

- `🎉Cache: 396k/397k(99%)` —— prompt 缓存命中率,高是好事,所以保持绿色。
- `5h(25% 57m)` 和 `7d(38% 2d10h)` —— 各档速率限制的已用量,各自带重置倒计时。
- `📁 …` —— 工作目录,路径过长时从前面裁剪。

Claude Code 还没报上来的数据(比如首条消息前的速率限制)会直接省略 —— 布局始终不动。

## 为什么用着舒服

- **永远两行。** 不膨胀、不跳动,不会把你的输入框挤跑。
- **适配终端宽度。** 读取 `$COLUMNS`,放不下时优先丢次要的段,绝不换行。
- **只有一个进度条。** 只有上下文窗口用进度条,其余都简短好读。
- **温和、跟随主题的配色。** 标准 ANSI,跟着你的终端走 —— 不刺眼,不和你的配色打架。
- **小而快。** 单个约 310 KB 的 Rust 二进制,无运行时依赖。

## 配置

开箱即用,下面的都是可选项。

设 `CLAUDE_HUD_ONELINE=1` 可只渲染一行 —— 想给输入框下方的内置模式行留出空间时很有用。`$COLUMNS` 和 `$LINES` 由 Claude Code 提供(v2.1.153+),终端很矮时 HUD 会自动收成一行。

## 手动配置

不想用 `:setup` 的话,直接把状态栏指向二进制:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/绝对路径/claude-hud",
    "padding": 2
  }
}
```

Windows 上路径用正斜杠。若 Claude Code 版本早于 v2.1.153(不导出 `$COLUMNS`),改成从 tty 读宽度:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /绝对路径/claude-hud"
```

## 从源码编译

```bash
cargo build --release        # -> target/release/claude-hud
```

用模拟输入试跑:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

`scripts/build-all.sh` 会把全部平台交叉编译到 `dist/<triple>/`。

## 发版

推一个版本 tag,CI 全包 —— 编译 Windows、macOS(x64 + arm64)、Linux(x64 + arm64),把二进制提交进 `dist/`,并发布 GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## 许可证

[MIT](./LICENSE)
