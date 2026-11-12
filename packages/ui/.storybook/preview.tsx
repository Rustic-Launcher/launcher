import type { Decorator, Preview } from '@storybook/react-vite';
import { MotionConfig } from 'motion/react';
import { useEffect, type ReactNode } from 'react';
import { TooltipProvider } from '../src/components/Tooltip';
import { Toaster } from '../src/components/Toast';
import { applyTheme, PALETTE_IDS, type PaletteId, type ThemeMode } from '../src/tokens/palettes';
import './storybook.css';

function ThemeFrame({
  palette,
  theme,
  children,
}: {
  palette: PaletteId;
  theme: ThemeMode;
  children: ReactNode;
}) {
  useEffect(() => {
    applyTheme(palette, theme);
  }, [palette, theme]);
  return <>{children}</>;
}

const withTheme: Decorator = (Story, ctx) => {
  const palette = ctx.globals.palette as PaletteId;
  const theme = ctx.globals.theme as ThemeMode;
  return (
    <ThemeFrame palette={palette} theme={theme}>
      <MotionConfig reducedMotion="user">
        <TooltipProvider delayDuration={200}>
          <div className="rx-ambient text-fg min-h-screen p-8">
            <Story />
          </div>
          <Toaster closeLabel="Close" regionLabel="Notifications" />
        </TooltipProvider>
      </MotionConfig>
    </ThemeFrame>
  );
};

const preview: Preview = {
  decorators: [withTheme],
  globalTypes: {
    palette: {
      description: 'Palette',
      toolbar: {
        title: 'Palette',
        icon: 'paintbrush',
        items: [...PALETTE_IDS],
        dynamicTitle: true,
      },
    },
    theme: {
      description: 'Theme',
      toolbar: { title: 'Theme', icon: 'mirror', items: ['dark', 'light'], dynamicTitle: true },
    },
  },
  initialGlobals: { palette: 'ember', theme: 'dark' },
  parameters: {
    layout: 'fullscreen',
    controls: { expanded: true },
    a11y: { test: 'error' },
  },
};

export default preview;
