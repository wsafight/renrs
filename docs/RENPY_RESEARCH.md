# Ren'Py 上游研究记录

## 研究副本与许可证边界

- 本地路径：`references/renpy`
- 当前提交：`9ed7dd3`
- 提交日期：2026-09-04
- 上游仓库：Ren'Py 官方源码仓库

该目录只用于本地研究，不参与 RenRS 构建或分发。Ren'Py 主体采用 MIT 许可证，部分组件和依赖
有各自许可证，完整说明见上游 `sphinx/source/license.rst`。RenRS 不复制 Python/Cython 实现，
只记录架构思想、行为边界、用户工作流和测试启发，并使用独立 Rust 类型重新实现。

## 重点阅读区域

- `renpy/lexer.py`、`parser.py`、`ast.py`、`script.py`、`execution.py`：缩进语法、AST、节点身份和执行上下文。
- `renpy/display/scenelists.py`、`display/core.py`、`display/render.pyx`：显示列表、交互循环与渲染边界。
- `renpy/display/transform.py`、`atl.py`、`display/transition.py`：transform、ATL 和 transition 生命周期。
- `renpy/loadsave.py`、`rollback.py`、`persistent.py`：存档容器、回滚日志和跨存档持久状态。
- `renpy/translation/`：翻译身份、语言目录和字符串替换。
- `renpy/audio/`、`display/video.py`：媒体通道、队列和视频生命周期。
- `renpy/display/predict.py`、`loader.py`：预测加载和资源访问。
- `renpy/screenlang.py`、`renpy/sl2/`、`style.pyx`：screen language、UI AST 和 style 系统。
- `renpy/lint.py`、`editor.py`、`scriptedit.py`：作者诊断和编辑器工作流。

## 借鉴后的独立设计

### 脚本与执行

Ren'Py 的脚本节点既表达语义，也参与跳转和存档恢复。RenRS 保留“先解析再执行”的边界，但将
AST 编译为扁平指令；运行时使用下标，持久化使用 SHA-256 稳定指令 ID。这样 CFG 分析、无窗口
测试和版本迁移可以围绕紧凑的 `Program` 工作。

### 交互循环

Ren'Py 在执行上下文与界面 interaction 之间切换。RenRS 使用 `WaitState` 暴露对白、菜单、暂停
和结束；播放器只消费效果和输入，不拥有剧情控制流。音频也由 Runtime 发出事件，再由播放器适配
真实后端。

### 显示状态

Ren'Py 支持多命名层、displayable 树、camera、ATL 和复杂 transition。RenRS 当前保存一个背景、
按显式层顺序和层内 z-order 排列的命名立绘层，以及每个立绘的 transform；层可单独清空。
它不支持 layer camera 或任意 displayable，也不将这部分描述为完整 ATL 或 screen 支持。

### 存档与回滚

Ren'Py 保存对象图、回滚日志、截图和 JSON 元数据。RenRS 保存小型显式快照，不使用 pickle。
v7 快照记录执行位置、变量、舞台、动态调用帧、语言、历史和有界完整检查点；旧快照格式会被拒绝。
外层存档增加项目/内容身份、游玩时长、章节和 SHA-256。

已读状态与玩家设置不放入单个快照，而是作为项目级数据独立保存。这与 Ren'Py 的 persistent/
preferences 思路相近，但数据结构和兼容格式完全独立。

### 本地化

Ren'Py 将翻译节点和语言纳入脚本体系。RenRS 使用独立 `TranslationId` 与 JSON catalog，将翻译
身份从执行 ID 和已读 ID 中分离。语言选择先做显式和结构 fallback，再进行变量插值与 markup
解析，保证翻译可调整占位符位置。

### 项目与发行

Ren'Py loader 为目录和归档提供统一资源视图。RenRS 对应为 `ProjectSource`：目录用于创作与热
重载，带校验的 `.renrs` 用于只读发行。设置、已读和存档进入系统用户数据目录，因此发行归档
不需要可写。

## 明确差异

- RenRS 不执行 Python，不兼容 `.rpyc`、Ren'Py 存档或 Python 插件生态。
- RenRS 使用 `.rns`，迁移器只是离线静态子集转换器。
- RenRS 当前只有内置且可主题化的 UI，没有 screen language、完整 style 或 displayable 系统。
- RenRS transform 是受限属性动画，不是 ATL；命名立绘层不包含 Ren'Py 的 layer camera/displayable 模型。
- RenRS 支持可序列化的 fade/dissolve、预测加载、流式视频、Web 播放器和 Capacitor 移动构建；组合转场、Live2D、shader 和原生 Rust 移动渲染仍未实现。
- RenRS 回滚保存有界完整检查点，不实现 Ren'Py 的可回退对象差异日志和固定回滚全部语义。
- RenRS 音频覆盖 music/sound/voice、基础队列/淡入淡出和静态 music/sound 相对音量；不支持任意 mixer，播放器默认可听，自动测试保持静音。

完整状态和后续优先级见 [差距分析](RENPY_GAP_ANALYSIS.md) 与 [路线图](ROADMAP.md)。

## 后续研究规则

1. 固定并记录上游提交后再更新对比，避免“Ren'Py 当前行为”无法复现。
2. 每个借鉴点记录源模块、观察到的行为、RenRS 独立设计和不兼容边界。
3. 不复制实现代码；若未来必须引入上游资产或代码，先单独审查许可证和分发义务。
4. 新迁移规则必须有真实输入样本、结构化报告和失败路径，不以字符串猜测代替语法理解。
5. 视频等重媒体能力必须先选择可靠后端并验证三平台，再写入产品支持范围。
