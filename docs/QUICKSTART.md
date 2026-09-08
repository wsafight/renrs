# 快速开始

RenRS 是使用 Rust 编写的视觉小说引擎，使用自己的 `.rns` 剧情格式。先用自带项目运行一次，
再创建你的故事。当前版本为 `0.1.0-rc.1`，尚未发布稳定版；RC 之前的旧脚本与旧存档不保证兼容。

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

项目的 `.rns` 文件支持四空格缩进。下面是可独立运行的最小故事：

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
cargo run --bin renrs-debug -- test my-story my-story/routes.json
cargo run --bin renrs-build -- my-story dist/my-story
```

构建目标目录必须尚不存在。发行目录包含播放器和 `game.renrs` 资源归档；播放器必须针对
目标系统构建。Web 与移动端另有构建流程，见 [命令行工具](TOOLING.md) 和 [移动端发行](MOBILE.md)。

RenRS 不直接执行 `.rpy`、Python 或 Ren’Py 存档。现有作品可以先阅读 [迁移范围](MIGRATION.md)。
