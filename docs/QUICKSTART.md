# 快速开始

RenRS 是使用 Rust 编写的视觉小说引擎，使用自己的 `.rns` 剧情格式。先用自带项目运行一次，
再创建你的故事。当前版本为 `0.1.0-rc.1`，尚未发布稳定版；RC 之前的旧脚本与旧存档不保证兼容。

完成本页后，你会得到一个能通过检查、打开播放器并生成本机发行目录的项目。本文按首次使用顺序
编排；需要查具体语法时再进入 [脚本语言](SCRIPTING.md)，不必预先通读全部文档。

## 准备环境

安装 Rust 1.88 或更高版本，然后进入这个仓库。原生播放器需要可用的图形界面和音频输出设备。
Linux 编译还需要 ALSA 开发库，Debian / Ubuntu 可安装 `libasound2-dev`。

```sh
rustc --version
cargo build --bins
```

Web 播放器和工作区需要 Node.js；官方文档站需要 Node.js 22.12 或更高版本。
流式视频转换和原生播放额外依赖 FFmpeg，纯图片剧情无需安装。

## 运行示例

```sh
cargo run --bin renrs-check -- demo
cargo run --bin renrs -- demo
```

第一条命令检查语法、资源引用和控制流，第二条打开播放器。示例中的背景、角色、音频及翻译
随仓库提供。开发目录支持校验后热重载，存档与偏好写入系统用户数据目录。

## 创建一个故事

在尚不存在的目录中生成项目：

```sh
cargo run --bin renrs-init -- my-story --title "My Story" --id org.example.my-story
cargo run --bin renrs -- my-story
```

`config id` 决定游戏数据隔离标识，请为不同项目选择不同 ID。模板包含图片、中文翻译、
分支结局和路线测试。数据交互模板使用 `--template inventory`。

## 写下第一段对白

先打开生成的 `my-story/script.rns`。项目的 `.rns` 文件使用四空格缩进。下面是可独立运行的
最小结构参考；若直接替换模板剧情，也要同步更新模板自带的 `routes.json` 路线断言。

```text
config title "My Story"
config id "org.example.my-story"

define mira = character "Mira" color "#4CC9A0"
default visits = 0

label start:
    set visits = visits + 1
    mira "Welcome. This is visit {visits}."
    menu:
        "Stay a little longer":
            mira "There is another story to tell."
        "Leave":
            "Until next time."
    return
```

更多对白、变量、镜头和音频语法见 [脚本语言](SCRIPTING.md)。界面布局及控件见
[界面与交互](SCREENS.md)。

## 检查与构建

```sh
cargo run --bin renrs-check -- my-story
cargo run --bin renrs -- my-story
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run --bin renrs-build -- my-story dist/my-story
```

先检查项目，再打开播放器确认实际交互。`renrs-debug test` 会执行 `routes.json` 中的路线断言；
修改剧情分支后应同时维护这些断言。构建目标目录必须尚不存在。发行目录包含播放器和
`game.renrs` 资源归档；播放器必须针对目标系统构建。

## 接下来读什么

| 目标 | 文档 |
| --- | --- |
| 继续写对白、分支、变量、镜头和音频 | [脚本语言](SCRIPTING.md) |
| 定制菜单、存档页、HUD 和数据控件 | [界面与交互](SCREENS.md) |
| 使用图形工作区或组装 SDK | [工作区与 SDK](LAUNCHER.md) |
| 构建 Web、移动端或本机发行目录 | [命令行工具](TOOLING.md) 与 [发布契约](RELEASE.md) |
| 转换已有 Ren'Py 项目 | [从 Ren'Py 迁移](MIGRATION.md) |

RenRS 不直接执行 `.rpy`、Python 或 Ren’Py 存档。现有作品可以先阅读 [迁移范围](MIGRATION.md)。
