import { act, fireEvent, render } from '@testing-library/react';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import { HeroLoop } from './HeroLoop';

const FALLBACK = [
  { src: '/media/hero-480.webm', type: 'video/webm' },
  { src: '/media/hero-480.mp4', type: 'video/mp4' },
];
const HD = [{ src: 'https://cdn.test/media/hero/abc/hero-1080p-av1.mp4', type: 'video/mp4' }];

beforeAll(() => {
  // jsdom has no media playback.
  HTMLMediaElement.prototype.play = vi.fn(() => Promise.resolve());
  HTMLMediaElement.prototype.pause = vi.fn();
});

describe('HeroLoop', () => {
  it('plays the bundled clip alone without an HD source', () => {
    const { container } = render(<HeroLoop fallback={FALLBACK} playing />);
    const videos = container.querySelectorAll('video');
    expect(videos).toHaveLength(1);
    expect(videos[0]!.dataset.hero).toBe('fallback');
    expect([...videos[0]!.querySelectorAll('source')].map((s) => s.getAttribute('src'))).toEqual(
      FALLBACK.map((s) => s.src),
    );
  });

  it('fades the HD clip in once it plays, then drops the fallback', () => {
    vi.useFakeTimers();
    const { container } = render(<HeroLoop fallback={FALLBACK} hd={HD} playing fadeMs={500} />);
    const hd = container.querySelector<HTMLVideoElement>('video[data-hero="hd"]')!;
    expect(hd.dataset.shown).toBe('false');
    expect(hd.style.opacity).toBe('0');
    fireEvent.playing(hd);
    expect(hd.dataset.shown).toBe('true');
    expect(hd.style.opacity).toBe('1');
    expect(container.querySelector('video[data-hero="fallback"]')).not.toBeNull();
    act(() => vi.advanceTimersByTime(700));
    expect(container.querySelector('video[data-hero="fallback"]')).toBeNull();
    vi.useRealTimers();
  });

  it('keeps the fallback when the HD clip fails, and reports when nothing plays', () => {
    const onFail = vi.fn();
    const { container } = render(<HeroLoop fallback={FALLBACK} hd={HD} playing onFail={onFail} />);
    const hd = container.querySelector('video[data-hero="hd"]')!;
    fireEvent.error(hd.querySelector('source')!);
    expect(container.querySelector('video[data-hero="hd"]')).toBeNull();
    expect(container.querySelector('video[data-hero="fallback"]')).not.toBeNull();
    expect(onFail).not.toHaveBeenCalled();
    const sources = container.querySelectorAll('video[data-hero="fallback"] source');
    fireEvent.error(sources[sources.length - 1]!);
    expect(onFail).toHaveBeenCalledTimes(1);
  });
});
