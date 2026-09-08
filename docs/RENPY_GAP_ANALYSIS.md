# Ren'Py 差距分析

## 对比范围

本分析基于本地研究副本 `references/renpy` 的提交 `9ed7dd3`（2026-09-04）。重点阅读了
脚本/AST、执行上下文、显示列表、screen language、ATL、存档/回滚、本地化、音频、视频、
预测加载和发行相关模块。RenRS 只借鉴架构边界和用户工作流，不复制上游代码。

项目尚未上线；RC 前旧版本兼容和存档迁移不作为产品缺口。`0.1.0-rc.1` 起的格式策略见
[发布契约](RELEASE.md)。

状态含义：**完成**表示当前 P0/P1 验收范围已实现；**部分**表示只覆盖静态或受限子集；
**未实现**表示文档和迁移报告必须明确拒绝或列为待办。

| 能力 | RenRS 状态 | 当前边界 | 后续优先级 |
| --- | --- | --- | --- |
| 脚本解析与静态诊断 | 完成 | 多文件 `.rns`、源位置、跨文件校验、CFG 分析 | 按真实项目扩充诊断 |
| 对白、菜单、条件、跳转 | 完成 | 支持条件菜单、插值和基础富文本 | 增加更多文本标签与输入 |
| 标签参数与返回值 | 完成 | 位置/命名/默认参数、调用时默认值、动态参数作用域、`return expr`、`_return` | `*args` / `**kwargs` 和仅命名参数未纳入静态子集 |
| Python/store 语义 | 未实现 | 整数、布尔、字符串、列表、记录和确定性内置函数 | Python 生态兼容仍为非目标 |
| 图片声明与舞台 | 部分 | 静态 image、背景、全局 alias、可排序/清空的命名立绘层、camera、受限 layered image | layer camera、任意 displayable、Live2D、粒子仍缺失 |
| ATL 与 transform | 部分 | transform、easing、串行 timeline、独立 parallel 轨道；迁移器覆盖静态 ATL 和 master camera | 循环/参数化 ATL、layer camera、动态 layeredimage 仍缺失 |
| Transition | 部分 | 可序列化 fade、dissolve | 组合转场仍缺失 |
| Screen/style/UI | 部分 | 桌面/Web 共用 JSON 界面、滚动容器、数据控件、拖放和扩展按钮 | 任意 displayable、完整 screen language 仍缺失 |
| 音频 | 部分 | music/sound/voice，音乐队列、淡入淡出、静态 music/sound 相对音量 | 任意 mixer、同步、更多格式为 P2 |
| 视频 | 部分 | 图片帧或流式视频、可选 WAV 音轨，按音轨时钟暂停/恢复 | 多音轨、字幕轨与更多平台实测仍待补齐 |
| 阅读与无障碍 | 部分 | NVL、下划线、注音、RTL shaping、shaped cluster 换行、键盘控件、自助朗读、Web 语义状态 | 竖排、彩色 emoji、原生 OS 屏幕阅读器语义树仍缺失 |
| 本地化 | 完成 | JSON catalog、稳定 ID、alias、fallback、CLDR 复数、RTL shaping、字体族 | 完整 ICU 富文本和屏幕阅读器语义树仍缺失 |
| 存档 | 完成 | 快照 v7、异步存档、显式 ID 内容更新恢复、桌面/Web 容器交换与读档验收 | 旧格式迁移与云同步仍缺失 |
| 回滚/已读/自动/跳过 | 完成 | 有界检查点、独立 profile、回滚屏障 | 固定回滚与高级偏好同步仍缺失 |
| 资源归档与发行 | 部分 | 签名差量补丁、本地 app 和商店配置工具 | 发行凭据与外部验收待提供 |
| 编辑器工具 | 部分 | 独立 editor crate、LSP、VSCode 语法、Launcher、项目模板和 SDK 打包 | 可视化剧情创作和完整调试器仍缺失 |
| 预测加载与大项目优化 | 部分 | 后台媒体、增量编译、共享 Program/历史、有界解码、帧/RSS 测量 | 外部大型作品仍需验证 |
| Web/移动端 | 部分 | Rust/WASM、共用界面、Capacitor Android/iOS 构建与系统分享 | 移动包使用 WebView；原生 Rust 移动渲染、真机矩阵与商店验收仍缺失 |
| Live2D/粒子/3D/shader | 未实现 | 无对应运行时 | P2，按项目需求评估 |

## P0/P1 验收结论

P0 已覆盖“项目目录或归档 -> 检查/运行 -> 持久化 -> 构建发行”的闭环。归档中的脚本、
主题、字体、图片、音频和翻译均通过 `ProjectSource` 读取；存档与设置不写回只读游戏目录。

P1 已覆盖中型静态视觉小说所需的参数化调用、稳定本地化身份、可恢复 transform、三路音频状态、
工作区 LSP、shaped cluster 换行、Web 辅助语义和严格迁移报告。当前默认可听，自动测试保持静音；
听感仍需人工验收。第一方参考 fixture 是回归基线，不是外部作者采用证明。

这两个优先级的完成目标是“可靠的独立 Rust 引擎基线”，不是复刻 Ren'Py。Screen language、
完整 ATL、Python 生态、可视化创作、平台服务与外部发行验收仍是主要差距。
最新实现与测试范围见 [产品优化记录](PRODUCT_UPGRADES.md)。

## 后续增加功能的准入条件

- 新脚本语法必须同时更新 parser、validator、compiler、runtime、迁移器、LSP 和文档。
- 可持久化状态必须定义当前版本读写、热重载和回滚语义；不要求旧格式迁移。
- 新媒体格式必须使用可靠解码器并通过目标平台本地 release 门禁；无法解码时应明确报错。
- Ren'Py 迁移不确定时必须输出 `assumption` 或 `unsupported`，不得静默改变语义。
- 新 Rust 模块按单一职责拆分，`src/` 与 `tests/` 中任何 `.rs` 文件不得超过 500 行。
