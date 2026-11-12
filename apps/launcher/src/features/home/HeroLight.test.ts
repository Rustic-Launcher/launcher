import { describe, expect, it } from 'vitest';
import { batterySaver, shouldRun } from './HeroLight';

describe('Launchpad hero light', () => {
  const base = { reduced: false, running: false, hidden: false, saver: false };

  it('runs only when nothing asks it to stop', () => {
    expect(shouldRun(base)).toBe(true);
    expect(shouldRun({ ...base, running: true })).toBe(false); // an instance is running
    expect(shouldRun({ ...base, hidden: true })).toBe(false); // window hidden or minimised
    expect(shouldRun({ ...base, saver: true })).toBe(false); // battery saver
    expect(shouldRun({ ...base, reduced: true })).toBe(false); // reduced motion
  });

  it('treats a discharging battery at 20 % or less as battery saver', () => {
    expect(batterySaver(null)).toBe(false);
    expect(batterySaver({ charging: false, level: 0.2 })).toBe(true);
    expect(batterySaver({ charging: false, level: 0.5 })).toBe(false);
    expect(batterySaver({ charging: true, level: 0.1 })).toBe(false);
  });
});
