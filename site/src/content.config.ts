import {defineCollection} from 'astro:content';
import {glob} from 'astro/loaders';
import {pages} from './catalog';

const idFromEntry = ({entry}: {entry: string}) =>
  pages.find(page => page.source === entry)?.slug ?? entry.replace(/\.md$/, '').toLowerCase();

const docsEn = defineCollection({
  loader: glob({pattern: '*.md', base: '.generated/en', generateId: idFromEntry}),
});

const docsZh = defineCollection({
  loader: glob({pattern: '*.md', base: '.generated/zh', generateId: idFromEntry}),
});

export const collections = {docsEn, docsZh};
