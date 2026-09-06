# Cargo Workspace

工作区把编译、执行、项目存储和编辑器集成拆开。根包 `renrs` 组装原生播放器和命令行工具。

| Crate | 职责 | 直接内部依赖 |
| --- | --- | --- |
| `renrs-syntax` | AST、值、span、诊断、文本标记和本地化数据 | 无 |
| `renrs-compiler` | 解析、降低、编译后的 Program/ID、静态分析和目录提取 | syntax |
| `renrs-runtime` | 解释器、当前格式快照、热重载、回滚、配置、调试器和路线探索 | compiler |
| `renrs-project` | 项目组装、资源、归档、主题、界面和视频清单 | compiler |
| `renrs-editor` | 符号、引用、格式化、剧情图和 LSP 服务 | compiler, project |
| `renrs-web` | 共享运行时的 WASM 绑定 | runtime |
| `renrs`（根包） | 原生呈现、音频、存档仓储、发行物和 CLI 入口 | compiler, runtime, project, editor |

`Program` 是编译器与运行时的契约。运行时依赖编译后的模型；执行时不加载文件、
不初始化图形、也不调用解析器。编辑器集成可以在没有原生播放器和音频栈的情况下
构建和测试。项目资源规则由桌面加载、归档创建、监听、Web 发行和编辑器磁盘索引共用。

VS Code 的 JavaScript 扩展在 `editors/vscode-renrs`；其 Rust LSP 在 `crates/editor`。
浏览器 UI 在 `web`；只有 WASM 接口是 Rust。根库再导出播放器和 CLI 使用的模块路径。
预发布 API 和存储格式可以不兼容地变更。没有大一统的 core crate，也没有跨 crate
以源码路径互相包含。

```sh
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p renrs-compiler
cargo test -p renrs-runtime
cargo test -p renrs-editor
cargo build --bins
cargo build -p renrs-web --target wasm32-unknown-unknown --release
```

所有工作区包共用一份根 `Cargo.lock`。源码行数检查覆盖每个 Rust crate，不包括生成的
依赖或构建产物。
