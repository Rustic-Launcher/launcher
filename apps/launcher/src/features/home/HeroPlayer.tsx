/** The player's 3D model idling with equipped cosmetics (lazy chunk: three.js loads after first paint). */
import { PlayerViewer, type CosmeticVisual } from '@rustic/ui/three';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useAccounts } from '@/queries/core';
import { useCosmetics, useLoadout } from '@/queries/store';

export default function HeroPlayer() {
  const { t } = useTranslation('home');
  const cosmetics = useCosmetics();
  const loadout = useLoadout();
  const { data: accounts } = useAccounts();
  const active = accounts?.find((a) => a.active);
  const skin = active?.skin ?? null;
  const worn = useMemo<CosmeticVisual[]>(() => {
    if (!cosmetics.data || !loadout.data) return [];
    const ids = new Set(Object.values(loadout.data).filter(Boolean));
    return cosmetics.data.filter((c) => ids.has(c.id)).map((c) => c.visual);
  }, [cosmetics.data, loadout.data]);
  const names = cosmetics.data?.filter((c) => worn.includes(c.visual)).map((c) => c.name) ?? [];
  return (
    <PlayerViewer
      cosmetics={worn}
      animation="idle"
      yaw={-0.45}
      skin={skin}
      ambience="forest"
      label={names.length ? t('hero.playerWearing', { items: names.join(', ') }) : t('hero.player')}
    />
  );
}
