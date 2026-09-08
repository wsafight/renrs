# 0.1 发布契约与检查表

`release.json` 是脚本工具读取的发布契约，Rust crate 与编辑器包的版本必须和它一致。
`0.1.0-rc.1` 起冻结下表已有格式；对象增加可选字段属于兼容变更，删除字段、改变字段语义或
接受规则必须提高对应格式版本。项目尚未发布过稳定版，因此 RC 之前的脚本与存档不承诺兼容。

## 兼容矩阵

| 边界 | RC 版本 | 读取策略 | 破坏性变更 |
| --- | ---: | --- | --- |
| `.rns` 语言 | `0.1` | 同一 `0.1.x` 引擎解析；未知语法报带位置诊断 | 更新语言说明和迁移器，进入下一个 minor |
| 机器 JSON 协议 | `1` | 消费者先检查 `protocol_version`；允许新增可选字段 | 提高 `protocol_version` 并保留旧 schema |
| `screens.json` | `1` | 拒绝未知版本和字段 | 提高文件 `version` |
| `.renrs` 资源归档 | `1` | 校验 magic、版本、边界和 SHA-256 | 提高归档版本或提供显式重打包工具 |
| Runtime snapshot | `7` | 只接受当前版本 | 提高版本；旧开发存档可以丢弃 |
| 桌面/Web 存档容器 | `2` | 只接受当前版本并验证校验和 | 提高版本并同步两端解析器 |
| extension/composition | `1` | 拒绝未知版本 | 提高对应 manifest 版本 |
| 帧视频/流式视频 | `1` / `2` | 严格校验形态、版本及可选的本地化音轨/字幕轨 | 提高对应视频 manifest 版本 |

`tests/release_contract.rs` 检查发布版本、机器协议、snapshot、存档和界面格式与编译代码一致。
格式常量改变时必须先更新实现、fixture、兼容矩阵和升级说明，不能只改 `release.json`。

## 本地 RC 门禁

```sh
node scripts/verify-local.mjs
node scripts/verify-local.mjs --web
RENRS_FFMPEG=/path/to/full/ffmpeg node scripts/verify-local.mjs --media
node scripts/verify-local.mjs --editor
node scripts/verify-local.mjs --release
# 或一次执行全部
RENRS_FFMPEG=/path/to/full/ffmpeg node scripts/verify-local.mjs --full
```

core 包含 Rust fmt/Clippy/测试、Biome、strict TypeScript、协议单测、demo、产品 fixture、
30–60 分钟第一方参考 fixture 和路线验收。`--web` 不依赖 FFmpeg；`--media` 单独生成并验证
并行动画、帧视频和流式视频；`--release` 验证 release 二进制、原生 smoke、SDK 和 Capacitor
工程结构。每个平台的发布产物仍须在该平台重新执行 release 门禁。

## 外部发布门禁

- [ ] 项目所有者提供的真实中型作品完成创作、迁移、性能和发行验收。
- [ ] 发布者目标桌面系统上的打包播放器验收。
- [ ] Android 与 iOS 物理设备的媒体、生命周期、文件导入和系统分享验收。
- [ ] 发布身份、签名、公证、隐私清单、商店素材和商店审核。

这些项目依赖作品、设备或凭据；本地生成 fixture、模拟器和未签名构建不能替代它们。
