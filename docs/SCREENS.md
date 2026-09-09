# 声明式界面

项目根目录可放置 `screens.json`，替换 `main_menu`、`save`、`load`、`settings`、`history`，
以及 `dialogue`、`choices`，并叠加只读 `hud`。未声明的界面继续使用内置版本。目录热重载和 `.renrs` 归档均支持该文件。
这是 RenRS 的受限 JSON 格式，不执行 Ren'Py screen language 或任意代码。

## 最小示例

```json
{
  "version": 1,
  "styles": {
    "title": { "font_size": 48, "text_color": "#FFFFFF" }
  },
  "main_menu": {
    "bounds": { "x": 70, "y": 100, "width": 430, "height": 520 },
    "root": {
      "type": "column",
      "gap": 20,
      "children": [
        { "size": 160, "type": "text", "style": "title", "text": "{title}" },
        { "size": 64, "type": "button", "text": "New game", "action": "new_game" },
        { "size": 64, "type": "button", "text": "Settings", "action": "settings" },
        { "size": 64, "type": "button", "text": "Quit", "action": "quit" }
      ]
    }
  }
}
```

## 布局与样式

界面坐标基于 1280x720 逻辑画布，窗口按比例缩放并保留黑边。`bounds` 定义整个界面的区域。
`row` 和 `column` 使用 `gap`、`padding`、`children`；子元素的 `size` 是主轴长度，省略后平分
剩余空间。固定长度溢出、非有限数、空容器、超过 64 个直接子元素或超出画布都会被拒绝。
每个界面最多 256 个元素，嵌套深度最多 16。

`style` 引用顶层 `styles` 中的名称，可设置 `font_size`（12–72）、`text_color` 和
`background_color`。样式只应用于叶子控件，不继承；在行列容器上声明样式会报错。
文本会换行并在受限区域内缩小，最小字号下仍无法容纳的尾部会省略。对白正文使用独立的分页逻辑。

## 控件

| `type` | 字段 | 行为 |
| --- | --- | --- |
| `text` | `text` | 文本，支持当前剧情变量、`{title}`、`{chapter}` 插值 |
| `image` | `path` | 项目相对路径，等比完整显示图片 |
| `button` | `text`, `action` | 调用受控播放器动作 |
| `dialogue` | 无 | 对白区，只允许在 dialogue 界面且必须恰好一个；至少 240x140 |
| `choices` | 无 | 选项区，只允许在 choices 界面且必须恰好一个；至少 200x64 |
| `set` | `text`, `variable`, `expression` | 按钮执行确定性变量更新，仅限 dialogue/choices/settings |
| `slider` | `text`, `setting` | 编辑偏好，至少需要 200x64 的区域 |
| `toggle` | `text`, `setting` | 切换 `high_contrast`、`reduced_motion`、`wait_voice` 或 `self_voicing` |
| `input` | `text`, `variable`, `max_length` | 编辑已声明的字符串变量；需要已开始游戏 |
| `list` | `source`, `item_height`, `gap` | 有界显示、滚轮及翻页；每个界面最多一个列表 |
| `viewport` | `id`, `content_height`, `child` | 可嵌套滚动区域，原生和 Web 保持独立滚动位置并裁剪内容 |
| `data_list` | `variable`, `selected`, `item_height`, `label` | 从列表变量生成有界选择项，结果写入字符串变量 |
| `drag` / `drop` | `expression` / `variable` | 受限表达式拖放，触摸、鼠标和键盘走同一确定性赋值 |
| `extension` | `name`, `input`, `variable` | 调用纯 Rhai 模块并事务性写回结果 |
| `row` / `column` | `children`, `gap`, `padding` | 行列布局 |
| `hotspot` | `action`, `variable`, `expression` | 不可见点击区；可关界面、写变量或继续剧情 |

元素可加 `visible` 表达式，结果为假时不绘制。`screens.json` 的 `story` 表定义剧中界面，用 `show screen` / `hide screen` / `call screen` 调用；剧中界面只接受 `hotspot`、`button`、`text` 和 `image` 控件。

`setting` 支持 `text_speed`、`auto_delay`、`music_volume`、`sound_volume`、`voice_volume`。
三个音量的初始值分别为 0.6、0.8、1.0。设置保存到玩家数据目录。

列表来源支持 `manual_saves`、`quick_saves`、`auto_saves`、`history`、`languages`，以及 `saves`。
`saves` 在读档界面跟随当前分组，在存档界面使用手动槽位。`manual_saves` 在存档界面可以写入，
其余存档列表只读。损坏或不属于当前项目的存档不能加载。历史排版结果按宽度和字号缓存，
剧情、语言或主题变更后失效；仅绘制可见行。

动作包括：

- `new_game`、`continue`、`save`、`load`、`settings`、`history`、`collection`。
- `quick_save`、`quick_load`、`rollback`、`auto`、`skip`。
- `manual_saves`、`quick_saves`、`auto_saves`：打开并切换读档分组，配合 `saves` 列表使用。
- `close`：保存偏好并关闭当前界面；`quit`：退出播放器。

标题界面必须提供 `new_game` 按钮。HUD 不允许按钮、滑块、开关、输入或可写列表，仅支持文本、图片和只读历史。
界面由键盘焦点和鼠标操作；历史与长列表支持 Home/End/PageUp/PageDown。

## 完整示例与边界

[`examples/visual_screens.json`](../examples/visual_screens.json) 包含六种界面的组合示例，可生成试玩项目：

```sh
cargo run --example generate_visual_fixture -- target/visual-story
cargo run -p renrs-player --bin renrs -- target/visual-story
```

桌面与 Web 共用 Rust 校验后的布局；Web 在手机尺寸按叶子控件顺序重新排列。原生对白分页，Web 正文可滚动。
NVL 使用专门的阅读页。历史列表在 Web 按每页 50 条读取，避免长篇作品产生无界 DOM。
界面文案可使用项目 `ui.*` 翻译，常用中文标签有内置回退；诊断文本并非全部本地化。

```json
{"type":"set","text":"Pack map","variable":"bag","expression":"push(bag, \"map\")"}
```

变量必须已声明且更新前后类型一致；只有停在对白或菜单时可以执行。表达式或菜单条件求值失败会保留原变量、
profile 和检查点。成功更新进入当前检查点，后续存档及回退可恢复。没有任意宿主回调、拖拽、嵌套 viewport 或 Ren'Py displayable。
