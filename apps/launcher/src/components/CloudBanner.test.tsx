import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { setCloud, type CloudClient } from '@/cloud/client';
import type { CloudBanner } from '@/cloud/types';
import { CloudBanners, dismissKey } from './CloudBanner';

afterEach(() => {
  cleanup();
  localStorage.clear();
});

const banner: CloudBanner = {
  id: 'b1',
  severity: 'warning',
  lang: 'en',
  title: 'Maintenance tonight',
  body: 'The **store** pauses at 22:00.',
  dismissible: true,
  ends_at: null,
  updated_at: '2026-11-15T10:00:00Z',
};

function mount(list: CloudBanner[]) {
  setCloud({ banners: async () => list } as unknown as CloudClient);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CloudBanners />
    </QueryClientProvider>,
  );
}

describe('cloud banners', () => {
  it('shows the launcher banners and remembers a dismissal per version', async () => {
    mount([banner]);
    const b = await screen.findByTestId('cloud-banner');
    expect(b).toHaveAttribute('data-severity', 'warning');
    expect(b).toHaveTextContent('Maintenance tonight');
    fireEvent.click(screen.getByTestId('cloud-banner-dismiss'));
    expect(screen.queryByTestId('cloud-banner')).toBeNull();
    expect(JSON.parse(localStorage.getItem('rustic.launcher.banners.dismissed') ?? '[]')).toEqual([
      dismissKey(banner),
    ]);
  });

  it('an edited banner shows again; a banner that cannot be closed has no button', async () => {
    localStorage.setItem('rustic.launcher.banners.dismissed', JSON.stringify([dismissKey(banner)]));
    mount([{ ...banner, updated_at: '2026-11-15T11:00:00Z', dismissible: false }]);
    expect(await screen.findByTestId('cloud-banner')).toBeInTheDocument();
    expect(screen.queryByTestId('cloud-banner-dismiss')).toBeNull();
  });
});
