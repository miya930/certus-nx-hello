// 設計情報を、リポジトリの各所から src/content/docs に集める。
// ドキュメントは書いた場所に置いたまま GitHub でも読めるようにし、サイト用のコピーはビルドのたびに作り直す。
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, posix, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoDir = join(siteDir, '..');
const outDir = join(siteDir, 'src', 'content', 'docs');

const PUBLISHED_DIRS = ['projects', 'docs'];
const IMAGE_EXTS = new Set(['.svg', '.png', '.jpg', '.jpeg', '.gif', '.webp']);
const GITHUB_BLOB_URL = 'https://github.com/miya930/certus-nx-hello/blob/main/';

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      return name === 'build' ? [] : walk(path);
    }
    return [path];
  });
}

function isPublishedDoc(rel: string): boolean {
  return rel === 'README.md' || (rel.endsWith('.md') && PUBLISHED_DIRS.includes(rel.split('/')[0]));
}

// README.md はフォルダの URL に、それ以外の Markdown はファイル名のフォルダの URL になる。
function route(rel: string): string {
  return rel.replace(/(^|\/)README\.md$/, '$1').replace(/\.md$/, '/');
}

// GitHub で読むための相対リンクを、サイトの URL に書き換える。
// サイトに集めないファイルへのリンクは、GitHub のファイルを指すようにする。
function rewriteLinks(markdown: string, rel: string): string {
  return markdown.replace(/\]\(([^)\s#]+)(#[^)\s]*)?\)/g, (link, path: string, hash = '') => {
    if (/^[a-z]+:|^\//.test(path) || IMAGE_EXTS.has(extname(path))) {
      return link;
    }
    const target = posix.normalize(posix.join(posix.dirname(rel), path));
    if (target.startsWith('..')) {
      return link;
    }
    if (isPublishedDoc(target) && existsSync(join(repoDir, target))) {
      const href = posix.relative(route(rel) || '.', route(target) || '.');
      return `](${href ? `${href}/` : './'}${hash})`;
    }
    return `](${GITHUB_BLOB_URL}${target}${hash})`;
  });
}

// 最終更新日は、元のファイルを最後に変えたコミットの日付にする。
// まだコミットしていないファイルには付けない。
function lastCommitDate(rel: string): string | undefined {
  const date = execFileSync('git', ['log', '-1', '--date=format-local:%Y-%m-%d', '--format=%cd', '--', rel], {
    cwd: repoDir,
    encoding: 'utf8',
    env: { ...process.env, TZ: 'Asia/Tokyo' },
  }).trim();
  return date || undefined;
}

// Starlight はページのタイトルを frontmatter の title から取る。
// frontmatter のないドキュメントは、先頭の見出しを title に移す。
function toPage(markdown: string, rel: string): string {
  const frontmatter = markdown.match(/^---\n([\s\S]*?)\n---\n+/);
  const heading = markdown.match(/^# (.+)\n+/);
  let fields: string[];
  let body: string;
  if (frontmatter) {
    fields = [frontmatter[1]];
    body = markdown.slice(frontmatter[0].length);
  } else if (heading) {
    fields = [`title: ${JSON.stringify(heading[1])}`];
    body = markdown.slice(heading[0].length);
  } else {
    throw new Error(`${rel}: frontmatter の title も、先頭の "# " の見出しもない`);
  }
  const date = lastCommitDate(rel);
  if (date) {
    fields.push(`lastUpdated: ${date}`);
  }
  return `---\n${fields.join('\n')}\n---\n\n${rewriteLinks(body, rel)}`;
}

function copyDoc(source: string, target: string): void {
  const ext = extname(source);
  if (ext !== '.md' && !IMAGE_EXTS.has(ext)) {
    return;
  }
  mkdirSync(dirname(target), { recursive: true });
  if (ext === '.md') {
    const rel = relative(repoDir, source).split(sep).join('/');
    writeFileSync(target, toPage(readFileSync(source, 'utf8'), rel));
  } else if (source.endsWith('.drawio.svg')) {
    // draw.io の編集用データは数百 kB の content 属性に入っており、Astro が画像サイズを読み取れなくなるため、表示用のコピーでは外す。
    writeFileSync(target, readFileSync(source, 'utf8').replace(/ content="[^"]*"/, ''));
  } else {
    cpSync(source, target);
  }
}

rmSync(outDir, { recursive: true, force: true });

copyDoc(join(repoDir, 'README.md'), join(outDir, 'index.md'));

for (const top of PUBLISHED_DIRS) {
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
