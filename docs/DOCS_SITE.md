# 文档站部署

官方文档站位于 `site/`，由 Astro 生成静态页面，Pagefind 提供浏览器内全文搜索。
主要内容直接读取仓库 `docs/*.md`，目录与标题定义在 `site/scripts/catalog.mjs`。
代码示例、内部链接和原始文件下载在构建时处理，无需启动 Rust 或 Web 播放器。

## 本地开发

要求 Node.js 22.12 或更新版本。

```sh
cd site
npm ci
npm run dev
```

开发服务默认地址为 `http://127.0.0.1:4321`。完整全文索引在生产构建时生成；开发时
搜索仍可按页面标题定位文档。

## 检查生产构建

```sh
npm run check
npm run build
npm run preview
```

构建会验证站内页面、静态资源和锚点链接。测试使用真实浏览器覆盖桌面、手机、目录、搜索、
主题及代码复制。执行前安装 Chromium：

```sh
npx playwright install chromium
npm test
```

## 本地门禁与发布

文档站的质量门禁在本地执行，以 `npm run check`、`npm run build` 为准；需要浏览器回归时
再单独运行现有 `npm test`。本轮不新增 GitHub Actions CI。

现有 `.github/workflows/docs.yml` 只保留为文档产物和 GitHub Pages 的可选发布通道，不作为
本地验收的替代。需要发布时，在仓库 **Settings → Pages → Build and deployment** 中选择
**GitHub Actions**；仓库必须已推送到 GitHub。本地检查通过不代表线上发布成功。

## 站点地址与子路径

默认使用当前仓库推导 Pages 地址：项目仓库发布到 `/<repository>/`，
`<owner>.github.io` 仓库发布到 `/`。所有导航、图片、全文索引及页面资源都使用相同基路径。

可通过 GitHub 仓库变量覆盖部署位置：

| 变量 | 示例 | 用途 |
| --- | --- | --- |
| `DOCS_SITE` | `https://docs.example.com` | canonical 与 sitemap 的站点来源 |
| `DOCS_BASE` | `/` 或 `/renrs/` | 部署子路径 |

自定义域名还需配置 Pages 域名和 DNS；本站不预设实际域名。

## 添加文档

在 `docs/` 新增 Markdown，然后在 `site/scripts/catalog.mjs` 注册来源文件、路径、标题、摘要和分组。
文档之间继续使用相对 `.md` 链接，站点自动改写为页面地址。历史开发记录有独立标记，
当前能力以使用手册和 [当前开发进度](NEXT_PRODUCT_WORK.md) 为准。
