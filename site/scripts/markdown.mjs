import {visit} from 'unist-util-visit';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {pages} from './catalog.mjs';
import {deployment} from './deployment.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url)).replace(/[/\\]$/, '');
export function resolveLink(url, file) {
  if (/^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(url)) return url;
  const [relative, hash = ''] = url.split('#');
  const target = path.relative(root, path.resolve(path.dirname(file), decodeURI(relative)));
  const base = deployment().base.replace(/\/$/, '');
  const page = pages.find(page => target === `docs/${page.source}`);
  if (page) return `${base}/${page.slug}/${hash ? `#${hash}` : ''}`;
  if (target === 'README.md') return `${base}/`;
  if (target.startsWith('..') || path.isAbsolute(target)) throw new Error(`Link escapes repository: ${url}`);
  return `${base}/source/${target.split(path.sep).map(encodeURIComponent).join('/')}${hash ? `#${hash}` : ''}`;
}

export function docsMarkdown() {
  return (tree, file) => {
    const first = tree.children.findIndex(node => node.type === 'heading');
    if (first >= 0 && tree.children[first].depth === 1) tree.children.splice(first, 1);
    visit(tree, node => {
      if (['link', 'image', 'definition'].includes(node.type)) node.url = resolveLink(node.url, file.path);
    });
  };
}
