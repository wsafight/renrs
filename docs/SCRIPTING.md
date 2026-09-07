# RenRS 创作手册

## 项目结构

```text
my-game/
  script.rns
  story/
    chapter-1.rns
  images/
  audio/
  fonts/
  locales/
    zh-Hans.json
  theme.json
```

RenRS 递归读取非隐藏目录中的 `.rns` 文件，按相对路径排序并合并。所有文件共享配置、角色、
图片、显示层、默认变量和标签命名空间；项目必须且只能定义一个 `start` 标签。资源路径相对项目根目录，
绝对路径、`..` 和其他越界路径会被拒绝。

播放器和无窗口工具都接受目录或 `.renrs` 归档。只有目录模式监听脚本修改；归档是只读发行输入。

## 配置与声明

```text
config title "My Story"
config id "org.example.my-story"

define e = character "Eileen" color "#ef6a6a"
default score = 0
default player_name = "Reader"
image room = "images/room.png"
image eileen = "images/eileen.png"
layer effects order 50
```

`config id` 应使用稳定的 ASCII 反向域名或 slug，决定设置、已读状态和存档目录。发布后修改 ID
会让播放器把项目视为另一款游戏。未声明时从标题派生，只建议用于原型。

`default` 在新游戏开始时按声明顺序求值。颜色支持 `#RRGGBB` 和 `#RRGGBBAA`。静态 `image`
允许后续用名称代替资源路径：`scene room`、`show eileen`。`master`、`transient`、`screens`、
`overlay` 是内置显示层；自定义层用 `layer name order integer` 声明，order 越小越先绘制。

## 对白、插值与文字标签

```text
label start:
    e "Hello, {player_name}. Score: {score}."
    "Narration has no speaker."
    e "A {b}bright{/b} {color=#ef8b72}signal{/color}.{br}It is moving."
```

值类型包括整数、布尔值、字符串、列表和记录。对白中的 `{name}` 插入变量；集合显示为 JSON；`{{` 和 `}}` 输出字面花括号。
支持 `{b}...{/b}`、`{color=#RRGGBB}...{/color}`、`{color=#RRGGBBAA}...{/color}` 和
`{br}`，以及 `{u}...{/u}`、`{ruby=annotation}...{/ruby}`。注音限 1 到 64 个字符，不允许嵌套。
标签不计入打字机字符数。原生注音按正文片段宽度缩小；较长注音建议拆成短词。

`nvl on` 启用整页多段叙事，`nvl clear` 清页，`nvl off` 返回普通对白。页面边界随存档和回退恢复。
每页最多 256 段，长页仍可分页或滚动。原生无障碍设置提供自助朗读，F8 可切换；macOS 使用 `say`，
Windows 使用系统语音，Linux 需安装 `espeak-ng`。Web 使用浏览器 SpeechSynthesis，语音取决于系统安装情况。

## 舞台与 transform

```text
label start:
    scene room
    show eileen as hero at right onlayer effects zorder 10
    move hero to center over 0.4
    transform hero x 24 y -12 scale 1.1 rotate 5 alpha 0.9 over 0.5 ease in_out
    transform hero anchor 0.5 1 crop 0 0 600 900
    transform hero uncrop
    hide hero
    clear effects
```

`scene` 替换背景并清空立绘。`show` 接受静态图片名或引号路径；位置为 `left`、`center`、
`right`。`onlayer` 选择命名立绘层，`zorder` 是层内 32 位整数，数值小的先绘制；旧写法
`layer 10` 仍等同于 `zorder 10`。同 alias 的立绘在全舞台唯一并会被替换，`clear name`
只清空指定立绘层。层名、顺序、立绘和清层结果都会进入存档与回滚；内容热重载会刷新层顺序，
若删除仍有立绘的自定义层则事务式拒绝重载。

`transform` 可组合以下属性：

- `x` / `y`：相对基础位置的像素偏移。
- `scale`：`0.01..=20` 的统一缩放。
- `rotate`：角度。
- `alpha`：`0..=1`。
- `anchor x y`：两个 `0..=1` 的归一化锚点。
- `crop x y width height` / `uncrop`：纹理像素裁剪或取消裁剪。
- `over seconds`：动画时长；省略时立即应用。
- `ease linear|in|out|in_out`：插值曲线，默认 `linear`。

transform、位置 tween、fade 和 dissolve 都会进入快照、读档与回滚状态。当前不支持 Ren'Py 的完整
ATL、layer camera、背景层拆分或任意 displayable。

`timeline:` 块可串行组合 transform、move 和 pause；`transition dissolve seconds` 混合前后舞台。
`video "clips/name/clip.json" over seconds` 支持图片帧清单（v1）和 MP4/WebM 流式清单（v2），时长必须匹配清单。使用 `renrs-video input.mp4 my-project clips/name --stream` 生成流式版本；转换需 FFmpeg 与 FFprobe，原生流式播放需 FFmpeg，Web 使用浏览器视频播放。
转换器从输入提取可选 `audio.wav`，音轨位置驱动播放、暂停和读档。无音轨的旧清单继续可用；视频音量使用音效通道。
转换命令、示例和媒体限制见 [新增能力](UPGRADES.md#media-and-screen-controls)。

并行编排使用 2 到 16 条独立时间线；同一 alias 不能由多条轨道同时修改，每条最多 256 步：

```text
parallel:
    timeline:
        transform left_actor x 160 over 1.5
    timeline:
        pause 0.25
        transform right_actor alpha 0.5 over 1.25
```

轨道只接受 `transform` 和 `pause`，总时长为最长轨道。保存中途状态后，桌面/Web 均从保存进度恢复。
角色图片预合成见 [产品优化记录](PRODUCT_UPGRADES.md#character-composition)。

## 变量与表达式

```text
set score = score + 1
set ready = score >= 1 and not false
set greeting = "Hello, " + player_name
```

支持 `+ - * /`、比较、相等、`and`、`or`、`not` 和括号；`+` 可连接两个字符串。
表达式没有文件、网络、Python 或 Rust 调用能力。

```text
default bag = list("key")
default quest = record("done", false, "reward", 20)
set bag = push(bag, "map")
set quest = put(quest, "done", contains(bag, "map"))
set reward = get(quest, "reward")
```

内置函数：`list(...)`、`record(key, value, ...)`、`get(data, key[, fallback])`、`put(data, key, value)`、
`push(list, value)`、`remove(data, key)`、`len(data)` 和 `contains(data, value)`。
列表索引从 0 开始，记录键为字符串；`contains` 对记录检查键。更新返回新值，必须用 `set` 接住。
重复键、错误类型、越界访问会报错；`get` 可提供缺失值。内置函数结果与界面变量更新最多 4096 个值、16 层集合与 1 MiB 文本，
表达式最多 512 个 token、32 层括号。集合与普通变量一样进入存档及回退。

## 条件与菜单

```text
if score >= 2:
    e "High score."
elif score == 1:
    e "One point."
else:
    e "No points."

menu:
    "Continue" id "choice.continue" if ready:
        jump next_scene
    "Wait" id "choice.wait":
        e "Take your time."
```

每个 `menu` 至少声明两个选项。`if` 条件为假时选项不会显示；即使运行时只有一个选项可见，
仍按菜单处理。条件必须产生布尔值。

菜单可在头部声明与选项同时显示的旁白或角色提示，不会增加一次继续操作：

```text
menu "Choose a route":
    "Left":
        jump left
    "Right":
        jump right

menu e "What should we do?":
    "Stay":
        return
    "Leave":
        jump outside
```

## 标签参数与返回值

```text
label start:
    call add_score(score, amount=2)
    e "New score: {_return}."
    return

label add_score(current, amount=1):
    return current + amount
```

必需参数必须写在带默认值的参数之前，位置实参必须写在命名实参之前。命名参数可调整顺序；
编译器会拒绝未知、重复、缺失和过多的参数。只对未显式传入的参数计算默认表达式。

一次 `call` 的全部显式实参和实际使用的默认表达式，都会先基于调用方当前变量表求值，再按 label
声明顺序一次性绑定。因此后一个实参或默认表达式不会看到同一次调用中新绑定的参数。参数会动态
遮蔽同名变量，并在返回时恢复之前的值或未赋值状态。`start` 不能声明参数，`jump` 不能进入带
参数的 label；必须使用 `call`。

`return expr` 在参数恢复前求值，将结果写入 `_return` 后返回调用处。不带表达式的 `return`
不改变 `_return`。返回栈为空时结束游戏，标签末尾也会隐式结束或返回。

## 音频与暂停

```text
play music "audio/theme.ogg" loop fadein 0.5 volume 0.7
queue music "audio/next.ogg" volume 0.6 fadein 0.25
play sound "audio/click.wav" volume 0.5
voice "audio/line-001.wav"
pause 0.5
stop music fadeout 0.8
```

音乐、音效和语音是独立通道，首次运行默认音量分别为 `0.6`、`0.8`、`1.0`。`play music`、
`queue music` 和 `play sound` 可带一次 `volume 0..1`，该静态相对增益会与玩家的通道音量相乘并随音乐状态存档；
它不是任意 mixer。`voice` 通常放在对应对白前，对白推进时
自动停止。非循环 WAV/Ogg 音乐可在解析到实际时长后推进队列；无法确定时长时不会伪造时长。
音频状态随存档和回滚恢复。当前自动测试不试听声音。

## 稳定 ID 与本地化

对白可在语句前声明稳定翻译 ID，并登记旧 ID alias：

```text
@id "intro.hello" alias "chapter1.old_hello" e "Hello, {player_name}."

menu:
    "Continue" id "intro.continue":
        jump next_scene
    "Wait" id "intro.wait":
        return
```

未显式声明时编译器会生成结构化 ID。显式 ID 用于本地化和开发热重载；重命名时可用
`alias` 映射预览会话的位置。项目尚未上线，存档只接受当前脚本指纹，不要求跨版本恢复。
ID 限 1 到 128 个 ASCII 字符，可用字母数字、`_`、
`-`、`.`、`/` 和 `:`。

提取翻译：

```sh
cargo run --bin renrs-i18n -- extract game zh-Hans game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- update game game/locales/zh-Hans.json
cargo run --bin renrs-i18n -- check game game/locales/zh-Hans.json
```

catalog 示例：

```json
{
  "language": "zh-Hans",
  "fallback": "zh",
  "messages": {
    "intro.continue": "继续",
    "intro.hello": "你好，{player_name}。"
  }
}
```

播放器读取 `locales/*.json`。语言优先使用项目玩家设置；也可用 `RENRS_LANGUAGE=zh-Hans`
指定初始语言。缺失和空字符串翻译回退源文本。翻译文本先选择，再执行插值和 markup 解析。

## 主题与字体

项目根目录可提供 `theme.json`。支持字体路径、字号、颜色、高对比度、减少动画，以及主菜单、
工具栏、对白框和存档列表的固定布局。`font_path` 和 `font_fallbacks` 必须是项目内安全相对路径；
原生端按字形覆盖率选择字体，支持阿拉伯文、天城文和 CJK 内置备用字体，Web 端按同一顺序加载。
`music_volume`、`sound_volume`、`voice_volume` 分别控制三路混音默认值。完整字段可参考 `demo/theme.json`。

本地化 catalog 可在 `plurals` 下声明 `{ "count": "items", "forms": { "one": "...", "other": "..." } }`。
运行时按 CLDR 规则选择 zero/one/two/few/many/other，缺少具体形式时回退 `other`。

## 检查与热重载

```sh
cargo run --bin renrs-check -- path/to/my-game
cargo run --bin renrs-check -- path/to/game.renrs
```

检查覆盖缩进、语法、颜色、跨文件声明、标签和参数引用、资源、翻译相关身份、不可达剧情、
路径敏感未赋值变量和无交互立即循环。

目录播放器每 250 毫秒检查 `.rns` 变化。新脚本只有在解析、校验、编译、分析及稳定 ID 映射
全部成功后才事务式替换；失败时继续运行旧程序，也不会重新触发已经经过的音频命令。归档不热重载。

稳定 ID 与相对路径、标签和结构位置有关。移动文件、重命名标签或重排同级语句可能改变自动 ID；
要在修改剧情后保持预览位置，可为关键交互添加显式 ID。热重载不能可靠映射时重新开始预览；
持久化存档只支持当前脚本版本，验收不包含旧存档迁移。
