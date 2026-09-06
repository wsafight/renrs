export type Lang = 'en' | 'zh';
export type Copy = {en: string; zh: string};

export const ui = {
  en: {
    skip: 'Skip to main content',
    home: 'RenRS home',
    primaryNav: 'Primary navigation',
    mobileNav: 'Mobile navigation',
    footerNav: 'Footer navigation',
    docsNav: 'Documentation',
    toc: 'On this page',
    pager: 'Adjacent pages',
    previous: 'Previous',
    next: 'Next',
    source: 'Download source',
    search: 'Search',
    searchDocs: 'Search docs',
    searchEmpty: 'No matching docs.',
    theme: 'Toggle theme',
    menuOpen: 'Open menu',
    menuClose: 'Close menu',
    lang: 'Switch to Chinese',
    copy: 'Copy',
    copied: 'Copied',
    capabilities: 'Capabilities',
    script: 'Script',
    tools: 'Tools',
    ship: 'Ship',
    docs: 'Docs',
    quickStart: 'Quick start',
    status: 'Status',
    github: 'GitHub',
  },
  zh: {
    skip: '跳到主要内容',
    home: 'RenRS 首页',
    primaryNav: '主要导航',
    mobileNav: '移动端导航',
    footerNav: '页脚导航',
    docsNav: '文档目录',
    toc: '本页目录',
    pager: '相邻文档',
    previous: '上一篇',
    next: '下一篇',
    source: '下载原文',
    search: '搜索',
    searchDocs: '搜索文档',
    searchEmpty: '没有匹配的文档。',
    theme: '切换主题',
    menuOpen: '打开菜单',
    menuClose: '关闭菜单',
    lang: 'Switch to English',
    copy: '复制',
    copied: '已复制',
    capabilities: '能力',
    script: '脚本',
    tools: '工具',
    ship: '发行',
    docs: '文档',
    quickStart: '快速开始',
    status: '当前进度',
    github: 'GitHub',
  },
} as const;

export const githubRepo = 'https://github.com/wsafight/renrs';

export const pagesMeta = {
  home: {
    title: {en: 'RenRS - a visual novel engine in Rust', zh: 'RenRS - 用 Rust 编写的视觉小说引擎'},
    description: {
      en: 'RenRS is a Ren’Py-inspired visual novel engine written in Rust. Write .rns scripts, check them, then ship to desktop, web, and mobile.',
      zh: 'RenRS 是一套使用 Rust 独立实现、受 Ren’Py 启发的视觉小说引擎。用 .rns 写下对白与分支，先检查再运行，并发行到桌面、Web 与移动端。',
    },
  },
  docs: {
    title: {en: 'Docs - RenRS', zh: '文档 - RenRS'},
    description: {
      en: 'RenRS manuals, authoring guides, shipping notes, and development records.',
      zh: 'RenRS 使用手册、创作指南、发行说明和开发记录。',
    },
  },
  notFound: {
    title: {en: 'Page not found - RenRS', zh: '页面不存在 - RenRS'},
    description: {en: 'The requested page could not be found.', zh: '找不到请求的页面。'},
  },
};

export function langFromStorage(value: string | null): Lang {
  return value === 'zh' ? 'zh' : 'en';
}
