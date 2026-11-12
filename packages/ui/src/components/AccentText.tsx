import { Fragment } from 'react';

/**
 * A title with the website's serif-italic accent: `*word*` in the text becomes
 * `<em class="rx-accent">word</em>` (translations carry the markers, e.g. "Your *locker*").
 */
export function AccentText({ text }: { text: string }) {
  const parts = text.split(/(\*[^*]+\*)/g).filter(Boolean);
  return (
    <>
      {parts.map((p, i) =>
        p.startsWith('*') && p.endsWith('*') ? (
          <em key={i} className="rx-accent">
            {p.slice(1, -1)}
          </em>
        ) : (
          <Fragment key={i}>{p}</Fragment>
        ),
      )}
    </>
  );
}

/** The same text without the accent markers (document titles, accessible names). */
export function plainText(text: string): string {
  return text.replace(/\*/g, '');
}
