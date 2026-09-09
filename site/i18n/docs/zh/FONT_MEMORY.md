# 原生字体内存

2026-09-06 的原生字体后续把急切加载整份项目字体轮廓，改成按需栅格化字形。完整中文覆盖保留；
字体文件不变，也不做 subset。

## 原因与改动

先前原生路径通过 macroquad 的 fontdue 后端加载 NotoSansSC。fontdue 0.9.4 在构造字体时
会展开已映射字形轮廓。用实际发行依赖做的隔离探测：加载前 RSS 1.56 MiB，读入
10,589,136 字节字体后 11.81 MiB，解析 30,890 个已映射字符后 254.14 MiB。这标出了一处
大分配来源，而不是把进程内存归到实现语言。

`crates/player/src/player/text.rs` 的新路径用 ab_glyph 0.2.32 解析和栅格化，etagere 0.2.15 分配图集：

- 宽度测量读取 advance 和 kerning，不分配轮廓、位图或 GPU 图集。只有实际绘制的字形才栅格化。
- 项目字体字节移入自有 face，输入上限 32 MiB。内置回退借用静态字节。整份字体仍可用。
- 单个 1024x1024 RGBA 图集占 4 MiB，第一次绘制字形时分配。查找表最多 4096 项。位图是临时的，
  上传后释放；图集没有第二份持久 CPU 副本。
- 图集满时回收。待绘制内容在 texel 复用前通过安全 camera API 刷新，保持当前渲染目标。
  回收保留同一纹理，不堆积退役纹理。
- 异常大的字形缩小到最多 1020x1020 像素，再按原始尺寸绘制。临时位图/栅格缓冲与 4 MiB
  图集上限分开。
- 字体重载先解析校验再替换状态。成功交换后失效缓存；无效字体保留运行时。纹理在原生图形
  上下文关闭前释放。

全部原生 UI、对白、注音、下划线和历史文字走这条路径。现有排版、Unicode 揭示和存档行为保留。
Macroquad 内部仍有小的内置字体；项目 CJK 字体不再走那条路径。Web 继续用浏览器文本渲染，
不在这次原生改动里。

## 测量

同一台 Apple M3 Pro、macOS 26.5.2、资源、字体、夹具和采样方法，见
[更早的对比](PERFORMANCE_COMPARISON.md)。每行是三次运行的中位数；数字是测量窗口内的
峰值采样进程 RSS。改前数值是保留的更早运行，不与本轮交错。

| 场景 | RenRS 改前 | RenRS 改后 | 下降 | 本轮 Ren'Py |
| --- | ---: | ---: | ---: | ---: |
| 空闲对白 | 359.3 MiB | 127.5 MiB | 64.5% | 239.7 MiB |
| 移动角色 | 353.5 MiB | 127.2 MiB | 64.0% | 237.0 MiB |

改后 RSS 范围空闲 126.8-127.9 MiB，运动 127.0-127.2 MiB。Ren'Py 空闲 238.6-241.6 MiB，
运动 180.1-238.8 MiB；保留较低的运动那次。这些数字描述这份小型原生工作负载，不是所有项目、
全部 GPU 内存或跨平台行为。

空闲 RenRS 栅格化 66 个字形位图，保留 68 个查找项（含空字形）；运动栅格化 41、保留 42。
两者都用一份 4 MiB 图集。报告里的单次 reset 是初始字体安装，不是容量抖动。
更早的场景图片纹理仍单独占 3,115,200 字节。

CPU 中位数：RenRS 空闲 2.43%、运动 7.98%；Ren'Py 2.83% 和 12.39%（100% 表示一个逻辑核）。
RenRS 空闲 CPU 从 1.71% 到 4.38%；本轮不建立 CPU 改进结论。空闲时 RenRS 绘制 0 个 scene，
三段约八秒运动窗口绘制 525/481/480。Ren'Py 运动绘制 957/960/958。不同更新率、工具栏和内部
栅格分辨率仍是跨引擎对比的限制。

十二次运行都通过非空白和运动像素检查。截图发生在 CPU/RSS 测量窗口之后。新产物在：

```text
target/engine-comparison-verified/results-1788697453495/
```

二进制 SHA-256：

```text
before: 39ed6c6c1f99abe3cceb6e92f0ed021c94a2fcdf1f8ee7d82f797b8da37a45a8
after:  f6237c2f7c4ae7fda7ce5acedea14fb6e2ed8ca97b4940e3181e8d3c594355f2
```

更早的播放器保留在 `target/renrs-before-lazy-fonts`。工作区没有提交基线；这些标识二进制，
不是可复现的 git 修订。

## 验证

- 完整 Rust workspace/all-target 测试、格式化和严格 Clippy 通过。
- 测试覆盖 CJK em 尺寸、非空白字形栅格、空格、无效字体、过大字形边界，以及测量数千个未见过的
  CJK 字符而不填充字形缓存。重载测试事务式拒绝坏字体。
- 原生图集压力示例强制九次 reset，检查复用前后和字体替换后像素一致。默认 drawable 和播放器
  使用的离屏 camera 路径都通过。最新产物：`target/text-cache-canvas-verified/`。
- 产品和 NVL/注音原生冒烟通过，含中文 UI 和存读档，各 17 张非空白截图。产物：
  `target/smoke-lazy-font-product/` 和 `target/smoke-lazy-font-reading/`。检查了中文和注音截图。

在仓库根目录、交互桌面、以及更早对比夹具/官方 SDK 可用时复现：

```sh
cargo build --offline --release -p renrs-player --example text_cache_smoke
cargo build --offline --release --example inspect_frames
target/release/examples/text_cache_smoke target/my-text-cache-check
node scripts/compare-engines.mjs target/engine-comparison-verified target/renpy-sdk/renpy-8.5.3-sdk 3
```

渲染器仍是简单字形渲染，不是完整 shaping 引擎。这里没有加入 RTL、复杂脚本 shaping 和彩色 emoji。
图集回收会给有很多独特大字形的画面增加栅格工作；预算仍固定。真机和 Windows/Linux 验收仍开放。
