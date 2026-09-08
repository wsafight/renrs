# RenRS 工具链

除 `renrs` 播放器外，所有工具均为无窗口命令，可用于编辑器、CI 和发行流程。以下示例在源码
仓库中用 `cargo run` 调用；发行包中可直接使用同名二进制。

按目标选择命令即可，不需要顺序阅读整页：

| 目标 | 首选入口 |
| --- | --- |
| 检查或格式化项目 | [`renrs-check` / `renrs-fmt`](#检查与格式化) |
| 创建项目并开始编辑 | [`renrs-init` 与编辑器](#创建项目与编辑器) |
| 验证路线、查看覆盖或比较改动 | [项目检查与影响分析](#项目检查与影响分析)、[剧情调试](DEBUGGING.md) |
| 提取和维护翻译 | [`renrs-i18n`](#本地化) |
| 生成本机发行目录或资源归档 | [构建发行目录](#构建发行目录)、[资源归档](#资源归档) |
| 构建 Web、移动端或 SDK | [Web 与移动端](#web-与移动端)、[工作区与 SDK](LAUNCHER.md) |
| 执行发布前检查 | [当前版本验收](#当前版本验收)、[发布契约](RELEASE.md) |
| 迁移 Ren'Py 项目 | [Ren'Py 迁移](#renpy-迁移) |

## 检查与格式化

```sh
cargo run --bin renrs-check -- game
cargo run --bin renrs-check -- game.renrs
cargo run --bin renrs-fmt -- game
cargo run --bin renrs-fmt -- --check game
```

`renrs-check` 接受目录或 `.renrs`，执行加载、资源校验、编译和控制流分析。`renrs-fmt` 处理目录
中的 `.rns`，规范行尾空白、连续空行和文件末尾换行，不改变四空格语义缩进或删除注释；
`--check` 不写文件。

`renrs-check --json game` 使用 v1 机器协议。所有机器接口、错误码和退出码见
[机器接口协议](MACHINE_PROTOCOL.md)。

## 项目检查与影响分析

```sh
cargo run --bin renrs-inspect -- game
cargo run --bin renrs-impact -- baseline-game candidate-game
cargo run --bin renrs-impact -- --git game HEAD
```

`renrs-inspect` 输出角色、变量、标签、结局、资源、本地化、静态可达性和实际路线覆盖。
`renrs-impact` 编译并比较两个项目；`--git` 模式从指定提交建立只读基线，与当前候选目录比较。
报告覆盖路线结果、结局、翻译和存档结构风险。静态存档风险不是兼容保证，真实存档仍应使用
`renrs-accept --saves` 验证。

## 创建项目与编辑器

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
```

目标目录必须不存在。模板包含背景、立绘、两条路线、中文 catalog、主题和标题/HUD 界面。
安装本地 [VS Code 扩展](../editors/vscode-renrs/README.md) 后，可使用 LSP、项目诊断、资源预览、
运行、构建和路线测试命令；通过 `renrs.toolsPath` 指定工具目录。资源管理器中的 RenRS Project
面板还提供 Ren'Py 迁移、带原始 `.rpy` 位置的迁移诊断、项目检查、影响分析、剧情图、界面/主题、
Web 构建、发布验收和当前版本存档检查。迁移器不会为不确定规则提供自动修改。

剧情状态查看、录制、重放和有界分支探索见 [剧情调试](DEBUGGING.md)。
中型项目生成与 `renrs-bench` 的测量范围见 [规模与运行验证](VALIDATION.md)。

## LSP

```sh
cargo run --bin renrs-lsp
```

`renrs-lsp` 使用 stdio Language Server Protocol。初始化时加载工作区内非隐藏目录的 `.rns`，支持：

- 完整文档同步和即时语法诊断。
- 角色、标签、静态图片和变量文档符号。
- 全文格式化。
- 工作区定义、引用和重命名。
- 关键字与工作区符号补全。

发行前仍应运行 `renrs-check`，因为它会加载完整项目和资源并执行 CFG 分析。

## 剧情图

```sh
cargo run --bin renrs-graph -- game story.dot
```

输出 Graphviz DOT，节点是标签，边区分静态 `jump` 与 `call`。省略输出路径时写到标准输出。

## 本地化

```sh
cargo run --bin renrs-i18n -- extract game zh-Hans game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- update game game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- check game game/locales/zh-Hans.json
```

- `extract` 创建或重写指定语言的 catalog；重写前应由版本控制保护已有翻译。
- `update` 添加新 ID 的空翻译并保留 obsolete 项，避免静默丢失人工内容。
- `check` 列出缺失/空翻译和 obsolete 项；存在未翻译项时返回失败。

三个命令均接受目录或归档项目。播放器自动加载项目内 `locales/*.json`。

## 资源归档

```sh
cargo run --bin renrs-pack -- game game.renrs
cargo run --bin renrs-unpack -- game.renrs extracted-game
cargo run --bin renrs-check -- game.renrs
```

`.renrs` 包含版本化 JSON 清单和连续资源 payload。每项记录规范相对路径、偏移、长度和 SHA-256；
读取与解包拒绝越界路径、重复路径、越界数据和校验失败。隐藏目录不会打包；解包不会覆盖已有文件，
也拒绝目标目录、父目录或输出文件中的符号链接。

归档不是只用于解包的容器。播放器、检查器、本地化工具和构建器都可直接读取它；播放器归档模式
不启用热重载。

## 构建发行目录

```sh
cargo build --bin renrs --bin renrs-build
cargo run --bin renrs-build -- game dist/my-game
```

若 `renrs-build` 与播放器不在同一目录，可显式指定：

```sh
cargo run --bin renrs-build -- game dist/my-game --player target/debug/renrs
```

构建器先执行项目校验、编译和控制流分析，然后原子式生成：

```text
dist/my-game/
  renrs              # Windows 为 renrs.exe
  game.renrs
  renrs-build.json
  README.txt
  LICENSE-renrs.txt
  LICENSE-font-OFL.txt
```

目标目录必须不存在，建议放在项目目录外。生成的播放器从任意工作目录无参数启动时，
优先打开与播放器相邻的 `game.renrs`；显式传入项目路径可覆盖这个默认值。

## Web 与移动端

```sh
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- game dist/web --shell web/dist
node scripts/mobile.mjs dist/web dist/mobile
```

Web shell 构建需要 Node.js、已安装的 `web/` 依赖、Rust WASM target 和匹配版本的
`wasm-bindgen`。发行包已包含 `web-shell/`，可直接将它传给 `--shell`。
VS Code 中通过 `renrs.webShellPath` 指向该目录；源码开发时指向构建后的 `web/dist`。

移动端脚本生成 Capacitor 工程配置，随后在输出目录安装依赖并添加 Android/iOS 平台。
输出目录必须不存在。移动端运行 Rust/WASM WebView，完整环境要求、构建步骤和验收边界见
[移动端发行](MOBILE.md)。

## 当前版本验收

```sh
cargo run --bin renrs-accept -- game
cargo run --bin renrs-accept -- game --saves saved-games
```

验收器输出 JSON，检查路线断言和实际存档恢复；失败时返回非零状态。存档必须使用当前容器和
快照格式。脚本内容更新后，只恢复带显式 `@id`/`alias` 的活动位置；自动或已删除的位置会失败。
成功的存档检查会在 `compatibility` 中列出 alias 映射、新增 default 和丢弃的旧回滚点。
不接受 `--baseline`，也不迁移旧快照格式。当前策略见 [产品改进记录](PRODUCT_UPGRADES.md)。

## 角色预合成

```sh
cargo run --bin renrs-compose -- game character.json images/variants
```

按配置中的图层顺序生成命名 PNG 和 `images.rns`，目标是项目内尚不存在的资源目录。
这是构建时预合成，配置示例和限制见 [产品改进记录](PRODUCT_UPGRADES.md#character-composition)。

## Ren'Py 迁移

```sh
cargo run --bin renrs-migrate -- path/to/renpy/game migrated-game
cargo run --bin renrs-migrate -- --strict path/to/renpy/game migrated-game
```

迁移器不加载 Ren'Py、不执行 Python。它在转换后重新解析、校验、编译和分析，并把问题写入
`migration-report.json`。`--strict` 在存在 assumption、unsupported 或后验证诊断时以失败退出，
适合 CI。完整范围见 [迁移手册](MIGRATION.md)。

Launcher 可直接选择 Ren'Py 源目录和新的输出目录，迁移成功后自动注册项目；质量视图读取
`migration-report.json`，并可选择另一个已注册项目作为 `renrs-impact` 基线。

## 本地发布包

在每个目标平台本地构建 release 二进制，再用 `scripts/package-sdk.mjs` 组装该平台 SDK。
工具包包含：

- `renrs`、`renrs-check`、`renrs-inspect`、`renrs-impact`、`renrs-fmt`、`renrs-graph` 和 `renrs-lsp`。
- `renrs-i18n`、`renrs-migrate`、`renrs-pack`、`renrs-unpack` 和 `renrs-build`。
- `renrs-init`、`renrs-debug` 和 `renrs-bench`。
- `renrs-accept`、`renrs-web-build`、`renrs-video`、`renrs-compose` 和 `renrs-update`。
- `web-shell/`、`mobile.mjs` 和 VS Code 扩展 VSIX。
- `demo/`、`demo.renrs`、文档、README、引擎及内置字体许可证。

SDK 对应构建机器的平台。签名、公证和商店文件使用 `scripts/release.mjs`，仍需发行方凭据；
当前不包含安装器或网络自动更新。

## 工程质量门槛

```sh
node scripts/verify-local.mjs
npm run audit

cargo fmt --all -- --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo test --offline --workspace --all-targets
```

上述 core 门禁还会检查 Biome、demo、脚手架路线、产品和第一方参考 fixture、Web/Launcher 单测，
以及 Web、VS Code 与 Launcher 的 strict TypeScript 和生产构建。它也运行各 Node 工作区的
生产依赖审计和 `cargo audit --deny warnings`，因此本机需安装 `cargo-audit`。当前精确豁免
`RUSTSEC-2025-0035`、`RUSTSEC-2026-0192`、`RUSTSEC-2026-0206` 和
`RUSTSEC-2026-0249`：它们分别来自 Macroquad soundness、无维护的 ttf-parser/rustybuzz，
以及 Rhai 的 smartstring 传递依赖，当前均无可直接升级的修复版本。豁免不代表风险消失；
升级或替换对应渲染、文本和脚本依赖时应复核并删除，任何新 advisory 仍会使门禁失败。
扩展门禁可以单独或组合执行：

```sh
node scripts/verify-local.mjs --web      # 非媒体浏览器流程
node scripts/verify-local.mjs --media    # 并行动画、帧视频、流式视频；要求完整 FFmpeg
node scripts/verify-local.mjs --editor   # VS Code host 与 VSIX
node scripts/verify-local.mjs --release  # release/native/SDK/Capacitor wrapper
node scripts/verify-local.mjs --full     # 上述全部
```

完整依赖和 RC 兼容矩阵见 [发布契约](RELEASE.md)。

`tests/source_size.rs` 递归检查 `src/`、`tests/` 和各个 workspace crate 的源码，
任何超过 500 行的 Rust 文件都会使测试失败。
超过职责边界的模块应拆成同名目录下的子模块，并保持现有公共 API。

完整本地门禁还覆盖模板创建、路线断言、分支探索、VSIX 打包和浏览器流程。窗口自动验收用法
和当前本地验证范围见 [验收记录](VALIDATION.md)。
