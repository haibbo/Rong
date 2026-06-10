// @ts-check
import { defineConfig } from 'astro/config';

// GitHub Pages project site: https://lingxia-dev.github.io/Rong
// If you deploy to a user/org site or a custom domain, set `base: '/'`
// (and update `site` accordingly).
export default defineConfig({
  site: 'https://lingxia-dev.github.io',
  base: '/Rong',
  trailingSlash: 'ignore',
  // English at the site root, Chinese under /zh/ (see src/pages/zh/).
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'zh'],
    routing: { prefixDefaultLocale: false },
  },
  markdown: {
    shikiConfig: {
      theme: 'vitesse-dark',
    },
  },
});
