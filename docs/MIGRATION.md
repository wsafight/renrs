# 从 Ren'Py 迁移

RenRS 迁移器是离线源代码转换工具。它不加载 Ren'Py、不执行 Python、不读取 `.rpyc` 或 Ren'Py
存档，也不承诺转换后的行为完全一致。

```sh
cargo run --bin renrs-migrate -- path/to/renpy/game migrated-game
cargo run --bin renrs-migrate -- --strict path/to/renpy/game migrated-game
```

输入可以是一个 `.rpy` 文件或项目目录。输出目录必须为空或不存在。目录迁移会递归转换 `.rpy`、
复制普通资源，忽略隐藏目录、`cache`、`saves`、`tl` 和 `.rpyc`，并始终写出
`migration-report.json`。

## 自动转换范围

- `define config.name` 和静态 `Character(...)` 声明。
- 静态 label（含固定参数与默认值）、旁白、角色对白、菜单标题和无条件基础菜单。
- 静态 `scene`、`show ... at left/center/right`、`show ... as alias`、标准 `onlayer`、整数
  `zorder` 和简单 `hide`。
- 简单 `$ variable = expression` 与 `default variable = expression`。
- `if`、`elif`、`else`、静态 `jump`、带位置/命名参数的静态 `call` 和 `return <expr>`。
- 基础 `play music`、`play sound`（含静态 `volume 0..1`）、`stop music` 和 `pause`。
- `with fade` / `with dissolve` 按明确记录的假设转换为 `transition fade 0.5`。
- 对白中的简单 `[variable]` 转为 `{variable}`。

场景和立绘名称会匹配源项目图片文件名。例如 `scene bg room` 可匹配
`images/bg_room.jpg` 或 `images/bg room.png`。找不到时生成 `images/bg_room.png` 假设路径，并在
报告中记录 `assumption`。裸 `scene black` 是一个确定性例外：没有同名真实资源时，迁移器会
生成 `images/black.png` 黑色图片，不记录假设。

## 明确需要人工迁移

- Python 块、`init python`、普通 `init` 和任意 Python 表达式。
- screen language、style、自定义 displayable 和 UI action。
- ATL/transform 块、动态 image expression、自定义 transition。
- label 的 `*args` / `**kwargs` / 仅命名参数、动态 jump/call，以及超出 RenRS 子集的参数表达式。
- 条件/动态菜单、复杂 Character、复杂插值。
- 自定义 layer、`behind`、camera、视频和插件语句；自定义层需手写 `layer name order integer`。
- Ren'Py label 的隐式 fallthrough；RenRS 在 label 末尾隐式 return/结束。

不支持的源行会保留为 `# TODO migration:` 注释，并在报告中记录 `unsupported`，不会尝试执行或
静默删除。固定 label 参数采用与 Ren'Py 一致的调用时默认值和返回时动态恢复；`start` 参数与
可变参数仍会拒绝。RenRS transform 仍需手写。

## 报告与严格模式

`migration-report.json` 包含：

- `converted_files`：转换的脚本数。
- `copied_resources`：复制的普通资源数。
- `generated_resources`：迁移器生成的确定性资源数，例如缺失的内置黑场图片。
- `issues`：带文件、行号和 `assumption` / `unsupported` 类型的转换问题。
- `post_validation_diagnostics`：转换后重新解析、资源校验、编译和 CFG 分析产生的诊断。

默认模式即使存在 issue 也会生成结果和报告，方便逐步修复。`--strict` 仍先写完结果与报告，
但只要 `issues` 或后验证诊断非空就返回失败，适合 CI 和批量迁移验收。

## 推荐复核流程

1. 在副本上迁移到新的空目录，不修改原 Ren'Py 项目。
2. 处理报告中的全部 `unsupported` 和 `assumption`。
3. 添加稳定的 `config id`，并给关键对白和菜单补显式翻译 ID。
4. 运行 `renrs-fmt`、`renrs-check` 和 `renrs-migrate --strict` 对照检查。
5. 对每个入口、菜单分支、返回路径和存读档点进行剧情验收。
6. 最后单独试听音频；默认音量为 `0`，自动测试不会判断听感。

迁移器只解决可证明安全的语法映射。完整功能差距见 [Ren'Py 差距分析](RENPY_GAP_ANALYSIS.md)。
