import { render, screen } from '@testing-library/react';
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
