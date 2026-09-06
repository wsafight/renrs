import {defineConfig} from 'astro/config';
import sitemap from '@astrojs/sitemap';
import {deployment} from './scripts/deployment.mjs';

export default defineConfig({
  ...deployment(),
  trailingSlash: 'always',
  output: 'static',
  integrations: [sitemap()],
  markdown: {
    shikiConfig: {
      theme: 'github-dark-default',
      wrap: false,
      langAlias: {rns: 'text', rhai: 'javascript'},
    },
  },
});
