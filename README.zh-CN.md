# RenRS

[English](README.md) · **中文**

[文档站](https://wsafight.github.io/renrs/?lang=zh)
· [快速开始](https://wsafight.github.io/renrs/start/quickstart/?lang=zh)
· [English docs](https://wsafight.github.io/renrs/)

RenRS 是用 Rust 独立实现、受 Ren'Py 启发的视觉小说引擎。用 `.rns` 写剧情，先检查再运行，
可生成本地播放器或只读归档。它不执行 Python，不运行 `.rpy`，也不读取 Ren'Py 存档。

> 候选版本 `0.1.0-rc.1`。机器协议、归档和界面 v1 契约已经冻结。存档必须使用当前容器与快照格式；内容更新后恢复还要求
> 所有活动执行位置都有显式稳定的 `@id`/`alias`。

## 运行示例

需要 Rust 1.88+。Linux 还需要 ALSA（Debian / Ubuntu 安装 `libasound2-dev`）。

```sh
cargo run --bin renrs-check -- demo
cargo run -- demo
```

播放器接受项目目录或 `.renrs` 归档。目录模式在校验通过后热重载；归档模式只读。
存档和偏好写入系统用户数据目录，按 `config id` 隔离。

无项目参数时，依次查找播放器旁或当前目录的 `game.renrs`、`demo/`。

## 创建项目

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run -- my-story
```

模板含插图、中文翻译、双结局和路线测试。每个游戏使用不同的 `config id`。
数据交互模板是 `--template inventory`。

最小脚本：

```text
config title "My Story"
config id "org.example.my-story"

define mira = character "Mira" color "#4CC9A0"

label start:
    mira "Welcome."
    menu:
        "Stay":
            mira "There is another story to tell."
        "Leave":
            "Until next time."
    return
```

## 发行

```sh
cargo run --bin renrs-pack -- demo demo.renrs
cargo build --bin renrs --bin renrs-build
cargo run --bin renrs-build -- demo dist/signal-at-dusk
```

`renrs-build` 在新目录中写入播放器、`game.renrs`、清单和许可证。
Web 与移动端见 [命令行工具](https://wsafight.github.io/renrs/reference/cli/?lang=zh)。

## 文档

[文档站](https://wsafight.github.io/renrs/) 默认英文。加 `?lang=zh` 或点页面上的「中文」
可切换，选择会记在浏览器里。

| 主题 | 中文 | English |
| --- | --- | --- |
| 快速开始 | [站点](https://wsafight.github.io/renrs/start/quickstart/?lang=zh) · [源文](docs/QUICKSTART.md) | [站点](https://wsafight.github.io/renrs/start/quickstart/) |
| 脚本语言 | [站点](https://wsafight.github.io/renrs/guides/scripting/?lang=zh) · [源文](docs/SCRIPTING.md) | [站点](https://wsafight.github.io/renrs/guides/scripting/) |
| 界面 | [站点](https://wsafight.github.io/renrs/guides/screens/?lang=zh) · [源文](docs/SCREENS.md) | [站点](https://wsafight.github.io/renrs/guides/screens/) |
| 命令行 | [站点](https://wsafight.github.io/renrs/reference/cli/?lang=zh) · [源文](docs/TOOLING.md) | [站点](https://wsafight.github.io/renrs/reference/cli/) |
| Web 与移动端 | [站点](https://wsafight.github.io/renrs/shipping/mobile/?lang=zh) · [源文](docs/MOBILE.md) | [站点](https://wsafight.github.io/renrs/shipping/mobile/) |
| 与 Ren'Py 的差距 | [站点](https://wsafight.github.io/renrs/project/comparison/?lang=zh) · [源文](docs/RENPY_GAP_ANALYSIS.md) | [站点](https://wsafight.github.io/renrs/project/comparison/) |

另外：[VS Code 扩展](editors/vscode-renrs/README.md)、
[剧情调试](https://wsafight.github.io/renrs/guides/debugging/?lang=zh)、
[架构](https://wsafight.github.io/renrs/engine/architecture/?lang=zh)。
已冻结的兼容矩阵和 RC 门禁见 [发布契约](docs/RELEASE.md)。

## 能力

- 声明式 `.rns`：角色、对白、菜单、transform、音频、NVL
- 先检查再运行：资源、控制流、确定赋值、不可达剧情
- 可恢复状态：快照 v7、回滚、带校验和的存档、桌面/Web 交换
- `screens.json`、`theme.json`、JSON 翻译目录、`renrs-i18n`
- 无窗口工具：check、fmt、graph、LSP、debug、pack、build、migrate
- Web（WASM）与 Capacitor Android/iOS 打包

首次运行音量为音乐 `0.6`、音效 `0.8`、语音 `1.0`。已有静音偏好会保留。

## 目录

```text
crates/syntax         AST、值、诊断、文本、本地化
crates/model          编译产物与项目包契约
crates/compiler       解析、降低、静态分析
crates/runtime        执行、快照、回滚、调试器
crates/project        来源、资源、归档、主题、界面
crates/editor         LSP、符号、格式化、剧情图
crates/extensions     沙箱化确定性扩展执行
crates/web            共享运行时的 WASM 绑定
src/player/*          原生 UI、渲染、音频
src/migration/*       支持的 Ren'Py 静态子集
src/bin/*             无窗口 CLI 入口
```

源码和测试中每个 Rust 文件不超过 500 行（`tests/source_size.rs`）。
crate 边界见 [架构](docs/ARCHITECTURE.md)。

## 开发

```sh
node scripts/verify-local.mjs

cargo fmt --all -- --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo test --offline --workspace --all-targets

# 原生静音截图验收，输出目录必须不存在
cargo run -- demo --smoke-test target/demo-captures --window-size 800x600
```

`node scripts/verify-local.mjs --full` 还会验证 Web 浏览器流程、VS Code 扩展宿主、release
二进制和 SDK 打包；运行前需安装 Web/编辑器 npm 依赖、Rust WASM target、`wasm-bindgen` 和浏览器工具。

Ren'Py 研究副本在忽略目录 `references/renpy`，见 [上游研究](docs/RENPY_RESEARCH.md)。
内置字体按 SIL OFL 1.1 分发（`assets/fonts/OFL.txt`）。
