# 工作区与 SDK

Launcher 是本地项目工作区，复用 Rust 命令行工具完成检查、运行、剧情图输出和构建。
它提供项目列表、脚本编辑、任务日志及 SDK 路径配置。

## 从源码启动

```sh
cargo build --bins
npm ci --prefix web
node scripts/launcher.mjs --port 4185
```

打开 `http://127.0.0.1:4185`。服务只监听回环地址，默认从 `target/debug` 读取工具，
项目和 SDK 路径记录在仓库 `.renrs/launcher.json`。可用 `--state` 指定另一份配置。

## 项目管理

已有项目使用目录路径注册，新项目使用“标准剧情”或“数据交互”模板。创建目录必须尚不存在。
从列表移除项目只删除工作区记录，保留项目文件。

脚本视图读取 `.rns` 文件。保存会检查载入时的文件指纹：文件被外部编辑后，必须重新载入，
避免覆盖别处的修改。保存成功后启动项目检查。

## 构建任务

工作区可启动本机发行包、Web 发行包和 `.renrs` 归档构建。各任务使用对应的 Rust CLI，
输出路径必须尚不存在；每次只运行一个任务，日志有大小上限，运行中的任务可以取消。
Web 构建需要预先生成 Web shell：

```sh
npm ci --prefix web
node scripts/build-web.mjs
```

`wasm32-unknown-unknown` target 和匹配版本的 `wasm-bindgen-cli` 需先安装，完整命令见
[命令行工具](TOOLING.md#web-与移动端)。

## 组装 SDK

```sh
cargo build --release --bins
node scripts/package-sdk.mjs dist/renrs-sdk target/release
```

打包前必须已生成 Web shell。SDK 包含工具、Launcher、Web shell、编辑器扩展源码及文档，
`sdk.json` 记录版本、目标平台与文件校验值。它仍要求本机安装 Node.js 22 或更新版本，
原生二进制对应构建机器的平台，不能当作全平台 SDK。

Launcher 与 SDK 已有实现，完整端到端验收仍在推进，见 [当前开发进度](NEXT_PRODUCT_WORK.md)。
