import { TooltipProvider } from '@rustic/ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { usePrefs } from '@/stores/prefs';
import { BedrockPanel } from './BedrockPanel';
import { EditionLogo } from './EditionLogo';
import { EditionSwitcher } from './EditionSwitcher';
import { JavaLaunchpad } from './JavaLaunchpad';

function wrap(ui: ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('edition switcher', () => {
  beforeEach(() => {
    act(() => usePrefs.getState().setLaunchpadEdition('rustic'));
  });

  it('starts on Rustic, says it is not affiliated, and remembers the choice', async () => {
    wrap(<EditionSwitcher />);
    expect(screen.getByTestId('edition-current')).toHaveTextContent('Rustic');
    expect(screen.getByTestId('edition-disclaimer')).toHaveTextContent(
      'Not affiliated with Mojang or Microsoft.',
    );
    const user = userEvent.setup();
    await user.click(screen.getByTestId('edition-switcher'));
    await user.click(await screen.findByTestId('edition-java'));
    expect(usePrefs.getState().launchpadEdition).toBe('java');
    expect(screen.getByTestId('edition-current')).toHaveTextContent('Minecraft: Java Edition');
    // Persisted with the other preferences (localStorage), so the next session opens on it.
    const saved = JSON.parse(window.localStorage.getItem('rustic.prefs') ?? '{}') as {
      state?: { launchpadEdition?: string };
    };
    expect(saved.state?.launchpadEdition).toBe('java');
  });

  it('loads the official icons at run time and falls back to a neutral icon', async () => {
    wrap(<EditionLogo edition="bedrock" />);
    const img = await screen.findByTestId('edition-logo-bedrock');
    expect(img.getAttribute('src')).toMatch(/^https:\/\/www\.minecraft\.net\//);
    fireEvent.error(img);
    expect(screen.getByTestId('edition-fallback-bedrock')).toBeInTheDocument();
  });
});

describe('java launchpad', () => {
  it('lists the profiles, launches one and streams its log', async () => {
    wrap(
      <div>
        <JavaLaunchpad />
      </div>,
    );
    expect(await screen.findByTestId('java-profile-java-fabric')).toHaveTextContent(
      'Fabric 1.21.1',
    );
    expect(screen.getByTestId('java-play-version')).toHaveTextContent('Fabric 0.20.1 · 1.21.1');
    fireEvent.click(screen.getByTestId('java-play'));
    await waitFor(
      () => expect(screen.getByTestId('java-play')).toHaveAttribute('data-action', 'stop'),
      {
        timeout: 5000,
      },
    );
    expect(
      await screen.findByText(/Setting user: SamCrafte/, {}, { timeout: 5000 }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('java-play'));
    await waitFor(() =>
      expect(screen.getByTestId('java-play')).toHaveAttribute('data-action', 'launch'),
    );
  });
});

describe('bedrock panel', () => {
  it('offers to play when Bedrock Edition is installed', async () => {
    wrap(<BedrockPanel />);
    expect(await screen.findByTestId('bedrock-play')).toHaveTextContent('Play Bedrock Edition');
    expect(screen.getByText(/version 1\.26\.6301\.0/)).toBeInTheDocument();
  });
});
