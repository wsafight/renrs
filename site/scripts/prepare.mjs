import {mkdir, readFile, writeFile, copyFile, stat} from 'node:fs/promises';
import {remark} from 'remark';
import {visit} from 'unist-util-visit';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {pages} from './catalog.mjs';
import {docsMarkdown} from './markdown.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url)).replace(/[/\\]$/, '');
const output = path.join(root, 'site/public/source');
const generated = path.join(root, 'site/.generated');
const processor = remark().use(docsMarkdown);

async function writeDoc(lang, page, value, sourcePath) {
  const rendered = await processor.process({path: sourcePath, value});
  const directory = path.join(generated, lang);
  await mkdir(directory, {recursive: true});
  await writeFile(path.join(directory, page.source), String(rendered));
}

for (const page of pages) {
  const source = path.join(root, 'docs', page.source);
  const tree = remark().parse(await readFile(source, 'utf8'));
  const copies = new Set([source]);
  visit(tree, node => {
    if (!['link', 'image', 'definition'].includes(node.type) || /^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(node.url)) return;
    const target = path.resolve(path.dirname(source), decodeURI(node.url.split('#')[0]));
    if (!target.startsWith(`${root}${path.sep}`)) throw new Error(`Link escapes repository: ${node.url}`);
    if (!pages.some(page => target === path.join(root, 'docs', page.source)) && target !== path.join(root,'README.md')) copies.add(target);
  });
  for (const file of copies) {
    if (!(await stat(file)).isFile()) throw new Error(`Expected source file: ${file}`);
    const destination = path.join(output, path.relative(root, file));
    await mkdir(path.dirname(destination), {recursive:true});
    await copyFile(file, destination);
  }
  const original = await readFile(source, 'utf8');
  const otherLang = page.sourceLang === 'zh' ? 'en' : 'zh';
  const translation = path.join(root, 'site/i18n/docs', otherLang, page.source);
  if (!(await stat(translation)).isFile()) throw new Error(`Missing ${otherLang} translation: ${page.source}`);
  await writeDoc(page.sourceLang, page, original, source);
  await writeDoc(otherLang, page, await readFile(translation, 'utf8'), source);
}
await mkdir(path.join(root, 'site/public/images'), {recursive:true});
for (const file of ['studio.png', 'mira.png']) await copyFile(path.join(root, 'demo/images', file), path.join(root, 'site/public/images', file));
const {version} = JSON.parse(await readFile(path.join(root,'site/package.json'),'utf8'));
await writeFile(path.join(root,'site/public/version.json'), JSON.stringify({version}));
