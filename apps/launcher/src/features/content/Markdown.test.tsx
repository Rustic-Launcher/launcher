import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Markdown } from './Markdown';

describe('Markdown', () => {
  it('renders headings, lists and paragraphs', () => {
    render(<Markdown source={'Intro text.\n\n## Features\n\n- One\n- Two\n\nEnd.'} />);
    expect(screen.getByRole('heading', { name: 'Features' })).toBeInTheDocument();
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('Intro text.')).toBeInTheDocument();
  });
  it('never interprets HTML', () => {
    const { container } = render(<Markdown source={'<img src=x onerror=alert(1)> hello'} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<img');
  });
});

describe('Markdown as the website writes it', () => {
  it('reads headings and lists without blank lines, and inline marks as text', () => {
    const { container } = render(
      <Markdown
        source={'# Rules\n- Be **kind**\n- No `cheats`\nSee [our site](https://example.org).'}
      />,
    );
    const view = within(container);
    expect(view.getByRole('heading', { name: 'Rules' })).toBeInTheDocument();
    const items = view.getAllByRole('listitem');
    expect(items.map((li) => li.textContent)).toEqual(['Be kind', 'No cheats']);
    expect(view.getByText('See our site.')).toBeInTheDocument();
  });
});
