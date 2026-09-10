# P0-P2 升级说明

实施清单见 [实施记录](IMPLEMENTATION.md)。Cargo 边界见 [Workspace 架构](ARCHITECTURE.md)。

## 当前存档与资源

运行时快照使用版本 8，放在版本 2 存档容器里，并兼容读取 v7。其他快照格式会被拒绝；不同的编译脚本指纹
只有在活动位置可通过显式 ID 或 alias 解析时才会接受。
项目尚未发布；不要求超出该兼容范围的旧存档迁移。当前存档保留调用点 ID、运行时状态和回滚历史，
带校验和。编辑器热重载用显式 `@id` 锚点单独重映射现场会话，并初始化新声明的 default。
见 [当前策略](PRODUCT_UPGRADES.md)。

`resources.json` 接受精确相对名或以 `/` 结尾的目录前缀的 `include` 和 `exclude` 列表。
它们不是 glob。隐藏路径分量、构建/缓存目录、临时文件和符号链接被排除。同一规则约束
目录加载、归档、监听和编辑器索引。

## 桌面播放器

播放器提供 60 个手动槽、三个快速存档和五个轮换自动存档。覆盖、读档、删除和退出需要确认。
退出写入恢复存档；脏进度也会定期以及在章节/选项/结束边界保存。存档操作使用有界后台队列
和同目录原子写入。槽位摘要缓存元数据、备注和缩略图。无效存档仍可见。

可选存档呈现记录对白页、揭示位置和剩余暂停/效果时间。导入/导出使用存档工具覆盖层中的
路径字段。Rodio 从缓冲文件或归档项流式播放 WAV/Ogg。资源打开和归档校验在 worker 上运行；
不需要完整 PCM 缓存或渲染线程注册音效。历史/自动模式仍可重放和完成语音。

偏好包括字号缩放、高对比度、减少动画和等待语音。项目 `ui.*` 翻译覆盖常用 UI 标签；
中文有内置回退。部分诊断和收藏标签仍是英文。这不是完整 UI 翻译。

`renrs-check --json <project>` 输出结构化诊断。VS Code 扩展增加工具发现和项目创建；
LSP 索引未打开的文件，并在编辑器缓冲关闭时重载磁盘版本。

## 持久进度

创建 `progress.json`：

```json
{
  "achievements": [{"id":"arrival","title":"Arrived","label":"ending"}],
  "gallery": [{"id":"view","title":"The view","label":"ending","image":"images/view.png"}],
  "endings": [{"id":"ending","title":"Completed","label":"ending"}],
  "rollback_barriers": ["ending"]
}
```

进入配置的标签会解锁对应项。该 profile 与存档分开，在新游戏、读档和回滚后仍在。
名称以 `persistent_` 开头的已声明变量也在这些操作后保留。屏障在进入其标签时清除先前
回滚检查点。桌面数据用 `profile.json`；Web 用 localStorage。损坏的桌面 profile 会在
该会话禁用 profile 写入，并保留原文件以便恢复。

## 媒体与界面控件

```rns
timeline:
    transform mira x 100 over 0.4 ease in_out
    transform mira alpha 0.5 over 0.4
scene "images/roof.png"
transition dissolve 0.5
video "clips/intro/clip.json" over 2
```

时间线是 transform、move 和 pause 的串行列表。`parallel` 组合独立的 transform/pause
时间线；这仍是 ATL 的子集。每个操作使用共享的可序列化等待模型。dissolve 保留上一舞台。
视频播放已校验的帧或流式清单。

```sh
RENRS_FFMPEG=/path/to/ffmpeg target/debug/renrs-video intro.mp4 my-story clips/intro
```

默认转换使用 FFmpeg 和 FFprobe，24 fps、宽度 640，最多 7,199 帧。加 `--stream` 用
FFmpeg/FFprobe 生成 v2 MP4 片段。使用转换器打印的时长。原生 v2 播放需要 FFmpeg；
v1 在没有它时仍可移植。Web v2 使用 HTML video。输入音轨变成清单引用的可定位立体声 WAV。
音轨时钟控制视频位置；暂停和读档恢复保留偏移。音乐和语音保持独立。预算、兼容性和测量见
[性能测量](PERFORMANCE.md)。

帧清单和流式清单都可以用最多 16 条本地化 WAV 音轨替代旧的 `audio` 字段，并内嵌最多
16 条字幕轨。播放器依次选择语言精确匹配、同一基础语言、唯一默认轨、无语言音轨和第一条轨道。
音轨音量与音效通道音量相乘。字幕 cue 必须按时间排序、互不重叠、不超出片段时长，除换行外
不得包含控制字符。

```json
{
  "version": 2,
  "fps": 24,
  "stream": {"path": "clips/intro/video.mp4", "seconds": 2, "width": 1280, "height": 720},
  "audio_tracks": [
    {"path": "clips/intro/en.wav", "language": "en", "default": true, "volume": 0.7},
    {"path": "clips/intro/zh.wav", "language": "zh-Hans", "label": "简体中文"}
  ],
  "subtitles": [
    {"language": "en", "default": true, "cues": [{"start": 0, "end": 1.5, "text": "Signal received."}]},
    {"language": "zh-Hans", "cues": [{"start": 0, "end": 1.5, "text": "信号已收到。"}]}
  ]
}
```

`audio` 与 `audio_tracks` 不能同时出现。这些可选字段不改变 v1/v2 清单版本；不兼容的结构变更
仍必须提升版本。

桌面 `screens.json` 增加 `high_contrast`、`reduced_motion` 和 `wait_voice` 的 `toggle`，
以及已声明字符串变量的 `input`：

```json
{"type":"input","text":"Name","variable":"player_name","max_length":24}
```

## Web 与调试

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.121 --locked --root target/web-tools
npm ci --prefix web
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- demo target/web-game
node scripts/serve-web.mjs target/web-game 4173
```

打开 `http://127.0.0.1:4173/`。WASM 和项目资源需要 HTTP 托管；IndexedDB 校验和需要
localhost 或 HTTPS。输出目录必须是新的。浏览器通过 `crates/web` 使用同一份 Rust 编译输出/
运行时。存档在 IndexedDB。导入/导出使用同一套已检查的桌面容器；Rust 处理原始快照，
不把大整数经过 JS number。

调试器包含 Cytoscape 剧情图、变量、调用栈、指令断点、单步，以及带项目指纹和 ID 的路线
覆盖导出。图边描述静态标签目标；它们不预测动态路径是否可行。

Web 支持自定义界面、主题颜色/字体、富文本、裁剪/锚点、NVL、并行动画和同步视频。
桌面分页和 Web 滚动仍是不同的阅读呈现。Capacitor Android/iOS 打包、生命周期和原生存档
分享见 [Web 与移动端](MOBILE.md)。

## 发行与更新

```sh
cargo build --release --bins
cargo build --release -p renrs-player --bin renrs
target/release/renrs-build demo target/distribution --player target/release/renrs
node scripts/release.mjs mac-app target/distribution target/Signal.app
node scripts/release.mjs store-files target/distribution target/store-files 12345 12346
target/release/renrs-update keygen target/update-signing
target/release/renrs-update create old.renrs new.renrs target/patch target/update-signing.key
target/release/renrs-update apply old.renrs target/patch patched.renrs target/update-signing.pub
```

把私钥放在游戏资源之外，并独立于补丁分发受信任公钥。补丁应用验证 Ed25519 签名、基础归档、
变更资源和重建归档。已有输出会被拒绝；失败时旧归档不动。没有网络自动更新客户端。

`mac-sign` 需要 `RENRS_MAC_IDENTITY`；`mac-notarize` 需要 `RENRS_NOTARY_PROFILE`。
`windows-sign` 需要 `RENRS_WINDOWS_CERT_SHA1` 和 Windows SDK。商店文件包含仅预览的 Steam VDF
和必须放在游戏根目录打包的 itch 启动器。签名、公证和上传需要发行方凭据。Steam 成就、
云存档、远程商店上传和外部 CI 验收尚未执行。

## 测量原生帧

```sh
target/release/renrs demo --profile target/demo-frames.json --window-size 800x600
```

这使用隔离存档数据、静音自动播放和第一选项。预热 60 帧，记录 300 帧，然后退出并给出
p50/p95/p99/最大帧时间、采样峰值 RSS 和常驻纹理字节。截图冒烟报告包含截图开销，不能当作
这个模式的游玩性能来对比。
