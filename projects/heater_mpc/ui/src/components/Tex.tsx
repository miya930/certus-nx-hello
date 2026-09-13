import katex from "katex";
import { useMemo } from "react";

interface Props {
  math: string;
  block?: boolean;
}

export function Tex({ math, block = false }: Props) {
  // 数式はこの画面に書いた固定の文字列だけなので、KaTeX の出力をそのまま埋め込む。
  const html = useMemo(() => katex.renderToString(math, { displayMode: block, throwOnError: false }), [math, block]);
  return block ? (
    <div className="tex-block" dangerouslySetInnerHTML={{ __html: html }} />
  ) : (
    <span dangerouslySetInnerHTML={{ __html: html }} />
  );
}
