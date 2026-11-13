import type { ReactNode } from 'react';

/** Inline marks the website writes (`**bold**`, `` `code` ``, `[text](https://…)`), as plain text. */
function inline(text: string): string {
  return text
    .replace(/\[([^\]]+)\]\((https:\/\/[^)\s]+)\)/g, '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/`([^`]+)`/g, '$1');
}

/**
 * Minimal, safe Markdown renderer for project and server descriptions: paragraphs, `#`/`##`/`###`
 * headings and `-` lists (also when they follow each other without a blank line, as the website's
 * form writes them); bold, code and links are shown as their text. No HTML is ever interpreted
 * (text nodes only), so descriptions cannot inject markup.
 */
export function Markdown({ source }: { source: string }) {
  const out: ReactNode[] = [];
  let para: string[] = [];
  let list: string[] = [];
  const flush = () => {
    if (para.length) {
      out.push(
        <p key={out.length} className="text-fg-muted text-sm leading-relaxed">
          {inline(para.join(' '))}
        </p>,
      );
      para = [];
    }
    if (list.length) {
      out.push(
        <ul
          key={out.length}
          className="text-fg-muted marker:text-accent-text flex list-disc flex-col gap-1 pl-5 text-sm"
        >
          {list.map((l, j) => (
            <li key={j}>{inline(l)}</li>
          ))}
        </ul>,
      );
      list = [];
    }
  };
  for (const raw of source.replace(/\r\n/g, '\n').split('\n')) {
    const line = raw.trimEnd();
    if (!line.trim()) {
      flush();
    } else if (/^#{1,3}\s/.test(line)) {
      flush();
      out.push(
        <h3 key={out.length} className="text-fg mt-2 text-sm font-semibold tracking-wide">
          {inline(line.replace(/^#{1,3}\s+/, ''))}
        </h3>,
      );
    } else if (/^\s*[-*]\s+/.test(line)) {
      if (para.length) flush();
      list.push(line.replace(/^\s*[-*]\s+/, ''));
    } else {
      if (list.length) flush();
      para.push(line.trim());
    }
  }
  flush();
  return <div className="flex max-w-3xl flex-col gap-3">{out}</div>;
}
