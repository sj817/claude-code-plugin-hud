# claude-code-plugin-hud

用 Rust 编写的 [Claude Code](https://code.claude.com/docs/zh-CN/statusline) 简约状态栏,显示模型、上下文窗口、花费、git 分支、速率限制和缓存状态。

[English](./README.md) · 简体中文

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

## 双行布局

第一行只保留模型底块,上下文文字和进度条不加底色,进度条后用 ` · ` 连接额度信息,最后显示费用。
第二行用普通文本显示缓存、版本/会话时长、分支和路径,不加底色块和连接箭头,路径固定在最右一项。
板块之间用细竖线 ` │ ` 分隔,模型和分支不加图标。

```text
 Opus 5 · high ▶ ctx 32% ━━━━━━ · 5h 24% 1h18m · 7d 38% 2d10h │ $12.84
Cache: 91%(47m) │ v2.1.257 · 1h42m │ main* │ › D:/Github/claude-code-plugin-hud
```

窗口变窄时先收起上下文刻度,再按优先级省略整段。额度和缓存优先于上下文和花费;
内容不换行,也不会靠填充空格推到终端最右边。
额度重置倒计时始终和百分比放在一起,紧凑显示时也不省略。第二行路径始终在最后;
放得下时保留完整路径,所有反斜杠统一显示为 `/`。路径前加小箭头 `›`,父目录用经典中性灰,项目名用清亮青色。
`origin` 指向 GitHub 时,路径通过 OSC 8 链接到仓库,省略后的路径也可点击。
在支持超链接的终端中按 Ctrl+点击(macOS 为 Cmd+点击)打开;支持 HTTPS 和标准 GitHub SSH 远程地址。
缺少或无法识别远程地址时保留普通路径。若 Claude Code 未识别终端的超链接能力,
可按[官方文档](https://code.claude.com/docs/zh-CN/statusline#可点击链接)在启动前设置 `FORCE_HYPERLINK=1`。
窗口过窄时先省略版本和时长,优先保留项目名,必要时再省略路径前部。
额度百分比表示**已用**额度:低于 50% 为青色,50–74% 为黄色,75–89% 为橙色,90% 起为红色。
网关消费额度仍可以超过 100%。

缓存命中率复用经典粉色(`theme::CACHE`),费用复用经典金色(`theme::COST`),分支用鲜艳蓝青色(`#00bfff`)。
`ctx`、额度/缓存倒计时、`Cache:`、版本号和父目录统一复用经典中性灰弱化样式;额度窗口标签和会话时长复用经典白色。
上下文百分比和进度条同步使用五档鲜艳颜色:低于 25% 蓝青、25–49% 绿、50–69% 黄、70–89% 橙、90% 起红。
额度时间不加括号。缓存显示为 `Cache: 99%(48m)`,百分比和括号间不留空格。
分支尾部的珊瑚红 `*` 表示有未提交修改。双行布局省略增删行数和 token 总量,下方的经典布局保留这些详情。
`CLAUDE_HUD_ASCII=1` 使用普通连接符、分隔符和刻度。
设置 `CLAUDE_HUD_STYLE=classic` 可切回经典布局。

## 经典布局

![经典布局](./demo.png)

```text
███░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.251 🚀 ⚡high(Opus 5)
Quota: 5h 25% 57m · 7d 38% 2d10h | 🎉Cache: 99% 47m | 📁 D:/Github/claude-code-plugin-hud
```

### 第一行

| 段 | 示例 | 含义 |
| --- | --- | --- |
| 上下文条 | `███░░░░░ 397k/1M(40%)` | 上下文窗口 已用 / 总量。唯一的进度条。低于 70% 绿色,低于 90% 黄色,90% 及以上红色。 |
| 行数变更 | `+2318 -922` | 本会话新增 / 删除的行数。 |
| 花费 | `💰 $27.62` | 本会话花费(美元)。 |
| 时长 | `⏱ 3h4m` | 自会话开始的挂钟时间。 |
| 分支 | `🌿 main*` | git 分支。末尾的 `*` 表示工作区有未提交改动。 |
| 版本 · 强度(模型) | `v2.1.251 🚀 ⚡high(Opus 5)` | 最后一个宽度感知段:Claude Code 版本、快速模式开启时的 🚀、推理强度、模型。 |

### 第二行

| 段 | 示例 | 含义 |
| --- | --- | --- |
| 速率限制 | `Quota: 5h 25% 57m · 7d 38% 2d10h` | 各时间窗的套餐额度用量,带重置倒计时。第二行的焦点:百分比按每档 25% 分四段着色 —— 低于 25% 绿色,低于 50% 黄色,低于 75% 橙色,75% 及以上红色。若你受网关消费额度约束(需 Claude Code v2.1.251+),会多出第三个窗口 `$ 63% 20d3h`;它是唯一可能超过 100% 的窗口。 |
| 缓存 | `🎉Cache: 99% 47m` | 本会话的 prompt 缓存命中率,以及缓存前缀还能保温多久(过期后显示 `cold`)。较旧的 Claude Code 不发送缓存统计,此时回退为 `396k/397k(99%)`,即最近一次响应的缓存占比。 |
| 目录 | `📁 D:/Github/...` | 工作目录,过长时从前面裁剪(但不会短于最后一段)。 |

说明:

- 高度始终两行。段不换行;当某一行超出 `$COLUMNS` 时,优先级最低的段会被丢弃。
- 缺失的数据直接省略,不会占位。首次 API 响应前没有速率限制;缓存段在首次响应前、以及未启用 prompt 缓存时同样不显示。
- 配色基于标准 16 色 ANSI,外加粉色 `Cache:` 标签、柔金色目录,以及额度刻度中的橙色档位。进度条和缓存的百分比保持无色。
- 不显示权限模式(auto/plan):状态栏 JSON 里没有这个字段。

## 配置

Setup 和安装器会将 `statusLine.refreshInterval` 设为 `30` 秒,让缓存和额度
倒计时在对话空闲时继续更新。已有用户需重新运行 setup/安装器,或在 `statusLine`
配置中添加 `"refreshInterval": 30`;仅更新二进制不会开启定时刷新。

两种布局默认都占两行。`CLAUDE_HUD_ONELINE` 可以只渲染第一行。设为 `1`(或 `true`):

```text
CLAUDE_HUD_ONELINE=1
```

当 `$LINES` 小于 10 时,两种布局也会收成一行。`$COLUMNS` 和 `$LINES` 由 Claude Code v2.1.153+ 提供;缺失时宽度回退为 80。

Claude Code 已经为状态栏提供了内置水平留白,所以安装器把额外的 `statusLine.padding` 保持为 `0`。HUD 还会在 `$COLUMNS` 内预留四列,给内置边距以及与状态栏共用一行的通知区。若你的终端需要别的值,用 `CLAUDE_HUD_MARGIN` 覆盖:

```text
CLAUDE_HUD_MARGIN=4
```

## 手动配置

想跳过 `:setup`,就把 `statusLine` 直接指向二进制:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-hud",
    "padding": 0,
    "refreshInterval": 30
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
