# 规模与运行验证

## 性能优化验收

P0-P2 性能优化后，134 项 Rust 常规测试、单独运行的 FFmpeg 解码/定位测试、
2 项 Web 音效生命周期测试和 7 项 Playwright 流程通过，fmt 与严格 Clippy 通过。
原生冒烟输出为 `target/perf-final-smoke`，原生流式视频也已进行窗口验证。
最新性能数据、预算定义与功能边界见 [PERFORMANCE.md](PERFORMANCE.md)。

## 工作空间拆分验收

2026-09-06 在下述同一台机器完成 workspace 拆分后的本地验证：125 项 Rust 测试、
fmt 和严格 Clippy 通过；VS Code 1.136.1 实际扩展宿主验证了定义跳转、关闭文件后的磁盘索引
和缺失资源诊断。VSIX 约 465 KB，不包含测试运行环境。

Web 的 6 项 Chromium 回归覆盖 1280x800 和 390x844 布局、存档备注与恢复、设置、
真实断点与单步、路线覆盖导出、收藏跨重启持久化、视频帧推进和弹窗暂停/读档恢复。
另验证首次播放被浏览器拒绝后，可在玩家点击时恢复音频。剧情图做了 canvas 像素检查。
截图位于 `web/test-results/`。

原生 demo 通过 17 张截图检查；长文本、自定义开关与输入控件项目在 release 模式通过全部
18 张截图检查和快速读档恢复。输出分别是 `target/p2-native-demo/` 与
`target/p2-native-visual-release/`。debug 长文本测试曾在截图阶段超过 45 秒，不能当作性能基线。

发布版 `--profile` 使用 800x600 窗口、静音、自动播放、60 帧预热和 300 帧采样，不读取截图：

| 项目 | p50 | p95 | p99 | 最大帧 | 采样峰值 RSS | 纹理峰值 |
| --- | --- | --- | --- | --- | --- | --- |
| demo | 8.29 ms | 9.36 ms | 16.65 ms | 18.08 ms | 711.5 MiB | 4.95 MiB |
| 10,000 段对白合成项目 | 8.33 ms | 9.34 ms | 10.62 ms | 14.09 ms | 392.3 MiB | 1.98 MiB |

原始报告为 `target/p2-demo-frames.json` 和 `target/p2-medium-frames.json`。这是短时窗口
的帧循环观测，未走完一万段对白；RSS 包含字体、音频及图形后端，明显高于资源纹理计数，
后续仍需内存归因和真实资源项目验证。没有旧版本同机对照，不宣称提速倍数。

本地发行目录和未签名 `target/Signal-P2.app` 已生成，plist 校验通过，并从 `/private/tmp`
无项目参数启动，通过 17 张截图和存读档检查，报告位于 `target/p2-app-smoke/`。Steam 配置使用
`preview=1`，itch 启动配置已生成。没有提交公证、上传商店或运行远程 CI。
签名差量补丁测试覆盖重建、内容篡改和错误公钥。

以下为本轮优化前的历史基线；当前功能和限制以 [新增能力](UPGRADES.md) 为准。

## 测量环境与范围

2026-09-06 本地实测：Apple M3 Pro、36 GiB 内存、macOS 26.5.2（arm64）、Rust 1.98.0，
Cargo release 配置启用 thin LTO。以下数值用于建立可复现的当前基线；没有同机旧版本对照，
不据此宣称整体提速倍数。

```sh
cargo build --release --bins
cargo build --release -p renrs-player --bin renrs
target/release/renrs-bench generate target/benchmark-story 100 100
target/release/renrs-bench target/benchmark-story 5
```

工作负载包含 100 章、10,000 段生成对白，另有结尾对白、长文本和每章两个分支；共有
10,805 条指令、209 个项目文件。图片使用独立路径，但内容复用两幅模板插图；这是合成项目，
不能代表真实高分辨率美术、大型音乐库或复杂条件图。

## 发布模式结果

| 操作 | 本机实测 |
| --- | --- |
| 目录检查与编译，5 次 | 83.79 / 56.22 / 74.74 / 57.15 / 56.64 ms |
| 首选项路线，10,102 次交互 | 52.32 ms |
| 快照创建与 JSON 序列化 | 5.62 ms，5,029,836 字节 |
| 快照恢复 | 6.07 ms |
| 写入一个完整存档（含校验和落盘） | 143.84 ms |
| 首次完整读取六个存档 | 263.18 ms |
| 已缓存列表，100 次均值 | 0.000448 ms/次 |
| 监听器实际空闲扫描，10 次均值 | 0.758 ms/次 |
| 目录归档打包 | 72.23 ms，3,292,092 字节 |
| 归档检查与编译 | 57.94 ms |

本地原始输出位于 `target/validation/benchmark.json`。编译测量包含资源与支持文件检查；
首次调用未主动清空操作系统文件缓存。运行测量只包含无窗口 Runtime，不包含 GPU、图片解码、
音频解码、帧循环和逐字动画。快照包含完整历史与最多 256 个回滚点。

以上为历史基线。当前播放器已通过后台存储队列读取和写入存档，限制图片解码队列的字节预算，
并使用流式音频替代完整解码缓存。最新帧间隔和 RSS 观测见性能记录；`renrs-bench` 的命令行
耗时仍包含同步落盘，不能直接视为播放器主线程停顿。

`renrs-bench` 也接受归档，但这时 `pack_ms` 表示复制归档，`watcher_idle_ms` 为 0。
生成器允许 1–200 章、每章 1–1000 段；测量器最多推进 100,000 次交互，过大或循环项目会报错。
生成目标必须不存在。

固定的第一方参考入口为：

```sh
target/release/renrs-bench generate-reference target/reference-story
target/release/renrs-accept target/reference-story
target/release/renrs-bench target/reference-story 5
```

它固定为 10 章、500 条对白、585 条指令和两条路线，预计阅读 30–60 分钟。该 fixture 用于发现
规模回归，不伪装成外部作者作品；真实创作、迁移和发行验收仍是独立门禁。

## 原生窗口验收

```sh
cargo run --example generate_visual_fixture -- target/visual-story
cargo run --bin renrs-debug -- test target/visual-story target/visual-story/routes.json
cargo run -p renrs-player --bin renrs -- demo --smoke-test target/captures-demo --window-size 1280x720
cargo run -p renrs-player --bin renrs -- target/visual-story --smoke-test target/captures-visual --window-size 800x600
```

测试输出目录必须不存在。模式使用输出目录下的 `data/`，三个音频通道固定静音，
由真实 App 状态与键盘动作驱动 Macroquad 渲染，等待当前图片可用后读取 framebuffer。
每张截图必须有至少 32 种采样颜色；运行错误、资源警告、快照恢复不一致或超时均以失败退出。
成功时输出 `report.json` 与 PNG，自动关闭窗口；不修改已有玩家设置或存档。

最多检查 14 个状态：标题、对白、第二页、首尾选项、历史首尾、设置、语言、翻译后的对白、
手动/快速/自动存档和快速读档。示例项目没有长对白时明确跳过第二页。用于该模式的项目必须
在 2,000 次推进内出现对白及其后的选择；总时限 45 秒。

本轮原生检查覆盖内置 demo 和可复现的自定义界面项目，窗口尺寸为 1280x720 与 800x600。
自定义项目包含六页对白、36 段额外历史、12 个长选项、中文 catalog、五个偏好滑块以及所有
可替换的界面。截图确认长文字与控件未重叠，中文与图片正确显示，小窗口保留等比黑边。
在本机 Retina 屏幕上，PNG 像素尺寸是请求窗口尺寸的两倍。

发行目录包含播放器、游戏归档、清单和两份许可证。从 `/private/tmp` 不传项目路径启动，
debug 与 release 播放器均完成截图和快速读档恢复，验证了基于播放器位置查找 `game.renrs` 的行为。
截图和报告保存在 `target/validation/`，可按 `demo-*`、`visual-*` 和 `package-*` 查看。

## 回归与交付范围

- 本机 fmt、全目标 check、严格 Clippy、109 项自动测试通过，所有 `src/` 和 `tests/` Rust 文件不超过 500 行。
- 测试覆盖旧快照/容器、稳定 ID alias、回滚、目录与归档一致性、损坏存档、迁移器、语言事务和路线断言。
- VS Code 扩展通过 strict TypeScript 检查并重新生成 VSIX，包含编译后的 LSP bundle、语法定义与许可证。
- Rust 检查、模板及路线测试、VSIX 打包、窗口截图和发行启动由本地分层门禁执行；GitHub Actions
  只保留文档站发布。

此历史阶段尚未实际运行远程三平台验收或人工音频试听。窗口自动验收没有模拟
完整的操作系统鼠标事件，也不能代替真实用户可用性测试。字体族/RTL、音频后台解码、存档截图、
精确恢复对白页码、移动端、代码签名与公证仍未实现。
