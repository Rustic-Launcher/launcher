import { useTranslation } from 'react-i18next';
import { MultiplayerList } from '../list/MultiplayerList';
import { MyServers } from '../list/MyServers';
import type { ServersContext } from '../ServersPage';

/** Mine: the servers hosted here (power, address) and the multiplayer list, as cards. */
export function MineTab({ ctx }: { ctx: ServersContext }) {
  const { t } = useTranslation('servers');
  return (
    <div className="flex flex-col gap-8">
      <section aria-labelledby="servers-joined" className="flex flex-col gap-3">
        <h2 id="servers-joined" className="text-base font-semibold">
          {t('list.sections.joined')}
        </h2>
        <MultiplayerList query={ctx.query} />
      </section>
      <section aria-labelledby="servers-hosted" className="flex flex-col gap-3">
        <h2 id="servers-hosted" className="text-base font-semibold">
          {t('list.sections.hosted')}
        </h2>
        <MyServers query={ctx.query} />
      </section>
    </div>
  );
}
