import type { ReactNode } from 'react';

/**
 * Minimal, safe Markdown renderer for project descriptions: paragraphs, `##`/`###` headings and
 * `-` lists. No HTML is ever interpreted (text nodes only), so descriptions cannot inject markup.
 */
export function Markdown({ source }: { source: string }) {
  const blocks = source.replace(/\r\n/g, '\n').split(/\n{2,}/);
  const out: ReactNode[] = [];
  blocks.forEach((block, i) => {
    const lines = block.split('\n').filter((l) => l.trim() !== '');
    if (lines.length === 0) return;
    const first = lines[0]!;
    if (/^#{2,3}\s/.test(first)) {
      out.push(
        <h3 key={i} className="text-fg mt-2 text-sm font-semibold tracking-wide">
          {first.replace(/^#{2,3}\s+/, '')}
        </h3>,
      );
      return;
    }
    if (lines.every((l) => /^\s*[-*]\s+/.test(l))) {
      out.push(
        <ul
          key={i}
          className="text-fg-muted marker:text-accent-text flex list-disc flex-col gap-1 pl-5 text-sm"
        >
          {lines.map((l, j) => (
            <li key={j}>{l.replace(/^\s*[-*]\s+/, '')}</li>
          ))}
        </ul>,
      );
      return;
    }
    out.push(
      <p key={i} className="text-fg-muted text-sm leading-relaxed">
        {lines.join(' ')}
      </p>,
    );
  });
  return <div className="flex max-w-3xl flex-col gap-3">{out}</div>;
}
