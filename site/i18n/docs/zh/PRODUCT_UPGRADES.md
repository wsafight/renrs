# 产品缺口工作

这是 Ren'Py 对比之后的第二轮产品迭代。它不复用更早工程或性能迭代的完成状态。

## 预发布策略

项目尚未发行。API、脚本和持久格式允许破坏性变更；向后兼容和旧版本迁移不是发行标准。
读档只接受当前容器和快照格式，且编译脚本指纹相同。当前构建的桌面/Web 存档交换、校验和、
回滚和事务式编辑器热重载仍支持。热重载重映射现场开发会话；它不是存档升级 API。

## P0

- [x] 共用可定制对白和选项界面。
- [x] Web 自定义界面、主题、样式文本、立绘裁剪和锚点。
- [x] 可移植的桌面/Web 存档交换和当前构建存档验收。
- [x] 首次运行默认可听、本地化播放器控件、键盘存档工具。
- [x] 发行验收报告和打包回归夹具。

## P1

- [x] 并行呈现，以及命名的预合成角色变体。
- [x] 视频音轨播放和同步。
- [x] NVL、注音、下划线、键盘控件和可选自助朗读。
- [x] 集成的 VS Code 项目和发行工作流。

## P2

- [x] Capacitor Android/iOS 工程、本地原生构建和平台适配。
- [x] 列表/记录、确定性内置函数和受控界面表达式。
- [ ] 真机验收、平台服务和商店上架。

## 外部验收

- [ ] 所有者提供的真实产品项目。
- [ ] 远程 Windows、macOS 和 Linux CI 结果。
- [ ] 发行方签名、公证和商店凭据。

未勾选工作仍开放。本地生成的示例是回归夹具，不是独立作者采用或商店批准的证据。

## 角色预合成

`renrs-compose <project> <character.json> <images/variants>` 创建 PNG 预设和 `images.rns`
声明文件。图层按列出顺序用 image crate 混合。输出必须是新的；资源路径和解码内存上限会校验。
这是构建时合成，不是动态 layeredimage、口型或 Live2D。每个图片声明共享项目全局命名空间。

```json
{
  "version": 1, "width": 800, "height": 1000,
  "layers": {
    "body": {"path":"characters/body.png"},
    "smile": {"path":"characters/smile.png","x":0,"y":0}
  },
  "presets": {"mira_smile":["body","smile"]}
}
```

## 发行验收

`renrs-accept game --saves saved-games` 为路线断言和每个当前构建存档槽输出 JSON。
失败以非零退出。不要求先前项目。共用原生/Web 存档容器有校验和保护，不是密码学真实性保证。
外部验收字段在独立验证前故意保持 false。先前 12/14 跨版本迁移报告不再是验收门槛。
另一脚本构建留下的开发存档可以丢弃，用新游戏替换。当前构建存档不要求显式结束锚点。
CLI 和 VS Code 不再暴露与旧版本的对比。

## 本地证据

- Rust workspace 测试和严格 Clippy；无音频设备的聚焦原声音频定位/暂停/重启测试；显式 FFmpeg worker 测试。
- 九条 1280 和 390 像素的 Chromium 流程：自定义 UI、含超出 JS 精度整数的可移植存档、变量动作、动画运动、NVL，以及两种音轨格式的暂停/存档/恢复。
- 原生自定义界面冒烟截图：`target/product-native-v2`。
- 原生 NVL/注音冒烟和读档恢复：`target/reading-native-captures`。
- VS Code 宿主回归和打包的 `editors/vscode-renrs/renrs-0.1.0.vsix`。
- Android APK：`target/mobile-product-v1/android/app/build/outputs/apk/debug/app-debug.apk`。
- iOS 模拟器应用：`target/mobile-ios-build/Build/Products/Debug-iphonesimulator/App.app`；
  安装/启动在 iPhone 17 模拟器，截图 `target/mobile-ios-screen.png`。
- 当前构建验收测试覆盖没有显式结束锚点的对白和完成存档，以及拒绝不同脚本指纹和损坏存档。
- 当前产品夹具验收通过两条路线和全部四个原生槽，命令为
  `renrs-accept target/product-story-v3 --saves target/product-native-v2/data/saves`。

最新预览 http://127.0.0.1:4183/product/ 演示自定义界面和数据；http://127.0.0.1:4183/reading/
演示动画、NVL、打字机进度和同步视频。阅读现在会随对话框和后台标签页暂停，从存档恢复可见字符
进度，并批量写入已读历史。验证和与 Ren'Py 的测量对比见 [性能对比记录](PERFORMANCE_COMPARISON.md)。

## 剩余边界

尚未加入 Python/Ren'Py 插件兼容、完整 ATL、camera、运行时 layeredimage、拖放、嵌套 viewport、
RTL shaping、独立启动器、云服务或商店集成。桌面分页和 Web 滚动区域不同；存档传递保留运行时
进度，不是相同的文字折行。语音取决于已安装的 OS/浏览器语音，不能替代完整无障碍树。
移动构建在 WebView 中运行 Rust/WASM；真机和原生分享流程仍需验收。见 [Web 与移动端](MOBILE.md)。
