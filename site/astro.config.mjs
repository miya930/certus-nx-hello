import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// GitHub Pages ではリポジトリ名のパスの下に公開される。
export default defineConfig({
  site: 'https://miya930.github.io',
  base: '/certus-nx-hello',
  integrations: [
    starlight({
      title: 'certus-nx-hello',
      defaultLocale: 'root',
      locales: {
        root: { label: '日本語', lang: 'ja' },
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/miya930/certus-nx-hello' },
      ],
      sidebar: [
        { label: 'はじめに', link: '/' },
        { label: 'プロジェクト', items: [{ autogenerate: { directory: 'projects' } }] },
      ],
    }),
  ],
});
