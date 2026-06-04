# claude-code-plugin-hud

一个用 Rust 编写的、**固定高度**的 [Claude Code](https://code.claude.com/docs/zh-CN/statusline) 状态栏 HUD。

市面上的 HUD 大多会铺好几行、在窄终端里换行错位、输入框一长就被挤跑。这个始终只占两行,按终端宽度自适应,配色温和并跟随你的终端主题。

[English](./README.md) | 简体中文

```
██░░░░░░ 311k/1M(31%) | +1820 -800 | 💰 $16.20 | ⏱  2h13m | 🌿 main*        v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 310k/311k(99%) | 5h(17% 2h19m) · 7d(37% 2d11h) | 📁 Github/claude-code-plugin-hud
```

## 显示内容

**第一行 —— 实时状态:**
`上下文进度条 已用/总量(%)` · `+新增/-删除 行` · `💰 花费` · `⏱ 耗时` · `🌿 分支(脏则带 *)`,右上角钉着 `版本` · `⚡强度(模型)`。

**第二行 —— 会话与额度:**
`🎉Cache: 命中/总量(命中率)` · `5h(已用% 重置) · 7d(已用% 重置)` · `📁 文件夹`。

## 设计原则

状态栏是内联渲染在输入框上方的,**不是固定悬浮层**,这一点无法改变。所以本 HUD 顺着这个约束设计:

1. **固定高度。** 永远两行(收缩时一行)。缺数据用占位符,绝不少一行,HUD 不跳动。
2. **按宽度自适应,绝不换行。** 读取 `$COLUMNS`,按优先级丢弃次要段直到每行放得下。换行会让高度翻倍 —— 正是要规避的。
3. **只有一个进度条。** 只有上下文窗口用进度条,其余都是紧凑文字。
4. **智能路径。** 路径在约 40 字符内显示全路径,超出则从顶层逐级裁剪(永不少于最后一级)。
5. **温和配色。** 标准 16 色 ANSI,跟随你的终端主题,外加少量刻意点缀。上下文按用量 绿→黄→红;缓存命中率相反(高=绿=健康)。

## 安装

本项目以 Claude Code 插件形式通过 marketplace 分发:

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/claude-code-plugin-hud:setup
```

`/claude-code-plugin-hud:setup`(插件命令以插件名作命名空间前缀)会检测你的系统/架构,从 `dist/` 选出对应的预编译二进制,并把 `statusLine` 写进你的 `settings.json`。(插件无法自行设置主 `statusLine`,只能由这类命令写入。)

### 手动配置

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/abs/path/to/claude-hud",   // Windows 上用正斜杠
    "padding": 2
  }
}
```

若 Claude Code 版本较旧(< v2.1.153)不导出 `$COLUMNS`,用下面这种包一层、从 tty 取宽度:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /abs/path/to/claude-hud"
```

## 配置项

| 环境变量                 | 作用                                          |
| -------------------- | ------------------------------------------- |
| `CLAUDE_HUD_ONELINE` | `1` 强制单行(给底部内置模式行留出空间)。                |
| `COLUMNS` / `LINES`  | 终端尺寸;Claude Code v2.1.153+ 会自动导出。          |

当 `LINES` 很小时,HUD 会自动收缩成一行。

## 编译

```bash
cargo build --release            # 本机 -> target/release/claude-hud
scripts/build-all.sh             # 所有目标平台 -> dist/<triple>/
```

release 配置针对体积调优(`opt-level = "z"`、LTO、`panic = "abort"`、strip)——二进制约 310 KB,无运行时依赖。

用模拟输入测试:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

## 发版

`git tag v0.1.0 && git push --tags` 触发 release 工作流,自动编译全部目标(Windows / macOS x64+arm64 / Linux x64+arm64)并附到 GitHub Release。若想让二进制随插件一起分发(这样 `/plugin install` 无需本地编译),把它们放到 `dist/<triple>/` 并提交。

## 许可证

MIT
