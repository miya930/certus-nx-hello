// 公開する設計情報を、リポジトリの各所から src/content/docs に集める。
// ドキュメントは書いた場所に置いたまま GitHub でも読めるようにし、サイト用のコピーはビルドのたびに作り直す。
// datasheets/ はメーカーの資料を変換したものなので、公開しない。
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoDir = join(siteDir, '..');
const outDir = join(siteDir, 'src', 'content', 'docs');

const IMAGE_EXTS = new Set(['.svg', '.png', '.jpg', '.jpeg', '.gif', '.webp']);

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      return name === 'build' ? [] : walk(path);
    }
    return [path];
  });
}

// Starlight はページのタイトルを frontmatter から取るため、先頭の見出しを title に移す。
function toPage(markdown, source) {
  const match = markdown.match(/^# (.+)\n+/);
  if (!match) {
    throw new Error(`${source}: 先頭に "# " の見出しがない`);
  }
  const title = match[1].replaceAll('"', '\\"');
  return `---\ntitle: "${title}"\n---\n\n${markdown.slice(match[0].length)}`;
}

function copyDoc(source, target) {
  const ext = extname(source);
  if (ext !== '.md' && !IMAGE_EXTS.has(ext)) {
    return;
  }
  mkdirSync(dirname(target), { recursive: true });
  if (ext === '.md') {
    writeFileSync(target, toPage(readFileSync(source, 'utf8'), relative(repoDir, source)));
  } else if (source.endsWith('.drawio.svg')) {
    // draw.io の編集用データは数百 kB の content 属性に入っており、Astro が画像サイズを読み取れなくなるため、表示用のコピーでは外す。
    writeFileSync(target, readFileSync(source, 'utf8').replace(/ content="[^"]*"/, ''));
  } else {
    cpSync(source, target);
  }
}

rmSync(outDir, { recursive: true, force: true });

copyDoc(join(repoDir, 'README.md'), join(outDir, 'index.md'));

for (const top of ['projects', 'docs']) {
  const dir = join(repoDir, top);
  if (!existsSync(dir)) {
    continue;
  }
  for (const source of walk(dir)) {
    const rel = relative(repoDir, source);
    const target = join(outDir, rel.replace(/README\.md$/, 'index.md'));
    copyDoc(source, target);
  }
}

console.log(`synced design documents into ${relative(siteDir, outDir)}`);
