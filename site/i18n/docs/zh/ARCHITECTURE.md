# Cargo Workspace

工作区把编译、执行、项目存储和编辑器集成拆开。根包 `renrs` 提供库和命令行工具；
原生播放器位于 `renrs-player`，仍输出 `renrs` 二进制。

| Crate | 职责 | 直接内部依赖 |
| --- | --- | --- |
| `renrs-syntax` | AST、值、span、诊断、文本标记和本地化数据 | 无 |
| `renrs-model` | 编译后的指令/ID 和面向运行时的项目包 | syntax |
| `renrs-compiler` | 解析、降低、静态分析和目录提取 | syntax, model |
| `renrs-runtime` | 解释器、当前格式快照、热重载、回滚、配置、调试器和路线探索 | syntax, model, extensions |
| `renrs-project` | 项目组装、资源、归档、主题、界面和视频清单 | syntax, model, compiler, extensions |
| `renrs-editor` | 符号、引用、格式化、剧情图和 LSP 服务 | syntax, compiler, project |
| `renrs-web` | WASM 绑定及浏览器边界的表达式解析 | syntax, compiler, runtime |
| `renrs`（根包） | 公共库 facade、存档仓储、发行物和 CLI 入口 | syntax, model, compiler, runtime, project, editor |
| `renrs-player` | 原生呈现、音频、字体和 `renrs` 播放器二进制 | 根包 `renrs` |

`CompiledProgram` 是纯脚本编译产物。`ProjectBundle` 在其上增加扩展、编译后的分层图像
和进度配置；`Program` 保留为兼容名称。运行时直接依赖该模型；执行时不加载文件、
不初始化图形、也不调用解析器。原生和 Web 适配层先把交互表达式解析为 `Expr`，再调用
运行时 API。编辑器集成可以在没有原生播放器和音频栈的情况下构建和测试。项目资源规则
由桌面加载、归档创建、监听、Web 发行和编辑器磁盘索引共用。

表达式边界与 Velin 0.5.1 共用：`renrs-syntax` 重导出其值、表达式、运算符、内置函数和
诊断类型，`renrs-compiler` 使用其有界 parser 与保守检查器，`renrs-runtime` 使用其参考
evaluator。兼容适配层在保存 AST 前移除表达式源码 span，保留 RenRS 字符串中方括号的
字面语义，并拒绝 `random` / `chance`。剧情语句、指令 ID、等待、热重载、回滚和持久化
仍由 RenRS 负责。

## Velin 所有权边界

| RenRS crate | 直接 Velin 依赖 | 负责内容 |
| --- | --- | --- |
| `renrs-syntax` | `velin-syntax` | 重导出 `Value`、`Expr`、字符串片段、运算符、内置函数和诊断 |
| `renrs-compiler` | `velin-parse`、`velin-check` | 解析并保守检查剧情、界面和分层图像表达式 |
| `renrs-runtime` | `velin-eval` | 执行表达式以及不可变 list/record 内置函数 |
| `renrs-extensions` | `velin` | 编译并执行有界、确定性的 `.velin` 纯模块 |

只有扩展 crate 依赖 Velin 的完整 facade 和 VM。其他 crate 只使用所需的最窄语言子 crate，
避免 syntax、compiler 或普通剧情求值意外引入完整 VM。

共享表达式路径覆盖 `.rns` 的默认值、label 参数默认值、`call` 实参、`return`、`set`、
扩展输入、分支/menu 条件，`screens.json` 的可见性和更新，分层图像条件、项目校验及调试器
求值。这些入口遍历同一份 Velin AST，并共享相同的值语义。

边界止于纯语言和纯计算。`.rns` parser 与视觉小说语句、稳定指令 ID、等待、热重载、回滚、
存档容器、界面布局、原生/Web UI、音频和渲染仍由 RenRS 负责。把它们迁入 Velin 会重复宿主
协议并模糊持久状态和呈现状态的所有权；只要这些边界仍然独立，它们就不是迁移候选。

VS Code 的 JavaScript 扩展在 `editors/vscode-renrs`；其 Rust LSP 在 `crates/editor`。
浏览器 UI 在 `web`；只有 WASM 接口是 Rust。根库是播放器和 CLI 使用的公共 facade。
图形和音频后端留在 `renrs-player`，所以 `renrs-check` 等 CLI 工具不再编译 macroquad
或 rodio。叶子 crate 不再导出其上游依赖中的 parser 和 syntax 模块。预发布 API 和存储
格式可以不兼容地变更。没有大一统的 core crate，也没有跨 crate 以源码路径互相包含。

```sh
cargo test --workspace --exclude renrs-player --all-targets
cargo test -p renrs-player --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p renrs-compiler
cargo test -p renrs-runtime
cargo test -p renrs-editor
cargo build --bins
cargo build -p renrs-player --bin renrs
cargo build -p renrs-web --target wasm32-unknown-unknown --release
```

所有工作区包共用一份根 `Cargo.lock`。源码行数检查覆盖每个 Rust crate，不包括生成的
依赖或构建产物。
