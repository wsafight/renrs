# 移动端发行

RenRS 把 Rust/WASM Web 播放器装进 Capacitor 8。这会生成 Android 和 iOS 工程；
它不会把 Macroquad 桌面渲染器移植到移动端。输出包含全部剧情资源，无需游戏服务器。

## 构建

```sh
node scripts/build-web.mjs
cargo run --bin renrs-web-build -- my-story target/web-story
node scripts/mobile.mjs target/web-story target/mobile-story
npm install --prefix target/mobile-story
npm run android:add --prefix target/mobile-story
npm run ios:add --prefix target/mobile-story
```

Android 需要 Node 22+、JDK 21 和 Android SDK 36。用 Android Studio，或在生成的
`android` 目录运行 `./gradlew assembleDebug`。把 `ANDROID_HOME` 设到已安装的 SDK。
发行签名属于原生工程；生成器拒绝覆盖已有项目。

iOS 需要 macOS 上的 Xcode。运行 `npm run ios --prefix target/mobile-story` 打开工程。
Capacitor 使用 Swift Package Manager，解析原生依赖时需要网络。未签名模拟器构建：

```sh
xcodebuild -quiet -project target/mobile-story/ios/App/App.xcodeproj \
  -scheme App -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath target/ios-build CODE_SIGNING_ALLOWED=NO build
```

更新时只用新的已校验 Web 构建替换 `www`，然后在移动工程里运行 `npm run sync`。
保持相同的 `config id` / Capacitor appId 和原生签名身份。不要为了发行设置重建整个工程。
项目仍是预发布：保留应用存储不保证能加载更早编译剧情的存档。脚本变更后请开始新游戏。
移动端标识必须是合法反向域名；分段使用字母、数字和下划线，不要用连字符。

## 原生行为

- Android 返回键关闭已打开面板或打开设置，并保留进度。
- 进入后台会暂停呈现并写入恢复槽；回到前台时面板保持可见，直到玩家继续。系统杀进程
  仍可能打断进行中的写入，所以这是对定期和手动存档的补充。
- 导出把已检查、与桌面兼容的 JSON 容器写到应用缓存，并打开系统分享面板。导入使用
  平台文件选择器。
- 播放内容遵守安全区。IndexedDB 和 localStorage 仍隔离在应用内；卸载可能清除它们。
  请导出重要存档。

生成器钉住 Capacitor core/platforms 8.5.1 和 CLI 8.4.3。后者避开 CLI 8.5.1 的
xcode/uuid 提示。提交生成的 package-lock.json，并用 `npm ci` 做可重复构建。

## 上架

提交前准备应用图标/启动图、版本/构建号、发行签名团队/密钥、商店元数据和相应隐私声明。
当前 Filesystem 插件使用文件时间戳：按 Capacitor Filesystem 文档，在 iOS 应用 target 的
`PrivacyInfo.xcprivacy` 中加入 `NSPrivacyAccessedAPICategoryFileTimestamp`，原因
`C617.1`。检查最终归档的清单和商店要求。生成的原生图标在发行方替换前仍是 Capacitor 默认图。

本地验证构建了 Android debug APK 和 iOS 模拟器应用，安装并启动了 iOS 应用，并检查了
渲染后的标题画面。桌面/移动浏览器测试覆盖剧情和存档流程。真机播放、原生分享/文件选择、
Android 生命周期、发行签名、商店审批、Steam 服务、云存档和内购仍需平台验收。
