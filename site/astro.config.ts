import { existsSync, readdirSync } from 'node:fs';
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// プロジェクトはフォルダごとに README.md を 1 つ持つ。
// サイドバーには、フォルダ名ではなく README.md の title を表示名として並べる。
const projectsDir = new URL('../projects/', import.meta.url);
const projects = readdirSync(projectsDir)
  .filter((name) => existsSync(new URL(`${name}/README.md`, projectsDir)))
  .sort();

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
      components: {
        PageTitle: './src/components/PageTitle.astro',
        LastUpdated: './src/components/LastUpdated.astro',
        MarkdownContent: './src/components/MarkdownContent.astro',
      },
      sidebar: [
        { label: 'はじめに', link: '/' },
        { label: 'プロジェクト', items: projects.map((name) => ({ slug: `projects/${name}` })) },
        ...(existsSync(new URL('../docs/', import.meta.url))
          ? [{ label: 'ドキュメント', items: [{ autogenerate: { directory: 'docs' } }] }]
          : []),
      ],
    }),
  ],
});
