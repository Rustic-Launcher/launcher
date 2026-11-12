import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LineChart } from './Misc';

describe('LineChart', () => {
  // Regression (QA): Horizon's first TPS sample (7356 against max 20) drew a line across the window.
  it('keeps every point inside the chart', () => {
    const { container } = render(
      <LineChart
        label="TPS"
        height={64}
        max={20}
        series={[{ id: 'v', values: [7356, 20, -5, Number.NaN, 19.5] }]}
      />,
    );
    const pts = container
      .querySelector('polyline')!
      .getAttribute('points')!
      .split(' ')
      .map((p) => Number(p.split(',')[1]));
    expect(pts).toHaveLength(5);
    for (const y of pts) {
      expect(y).toBeGreaterThanOrEqual(0);
      expect(y).toBeLessThanOrEqual(64);
    }
  });
});
