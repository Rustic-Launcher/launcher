/**
 * The Bedrock panel (ADR 0172): on Windows, Bedrock Edition is started when its Microsoft Store
 * package is installed, else the Store page opens; other systems read "Windows only".
 */
import { Badge, Button, Skeleton, toast } from '@rustic/ui';
import { ExternalLink, Play, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import { EditionLogo } from './EditionLogo';
import { EditionSwitcher } from './EditionSwitcher';
import { useBedrock } from './queries';

export function BedrockPanel() {
  const { t } = useTranslation('java');
  const { data, isLoading, refetch, isFetching } = useBedrock();
  const [pending, setPending] = useState(false);

  const act = async (cmd: 'bedrock_launch' | 'bedrock_open_store') => {
    setPending(true);
    try {
      await invoke(cmd);
      if (cmd === 'bedrock_launch') toast({ tone: 'success', title: t('bedrock.started') });
    } catch (e) {
      toast({
        tone: 'error',
        title: t('bedrock.failed'),
        description: e instanceof IpcFailure ? e.message : String(e),
      });
    } finally {
      setPending(false);
    }
  };

  return (
    <section
      aria-label={t('switcher.bedrock')}
      data-testid="bedrock-panel"
      className="border-border-default relative isolate flex aspect-[16/7] min-h-72 flex-col items-center justify-center gap-4 overflow-hidden rounded-2xl border px-6 text-center"
      style={{
        background:
          'radial-gradient(110% 90% at 80% 10%, rgba(64,110,160,0.42), transparent 60%), radial-gradient(90% 80% at 15% 95%, rgba(90,120,70,0.4), transparent 65%), linear-gradient(160deg, #18202b 0%, #141820 55%, #12141a 100%)',
      }}
    >
      <EditionSwitcher overlay />
      <div className="flex items-center gap-2.5 text-lg font-bold text-white">
        <EditionLogo edition="bedrock" size={28} />
        {t('switcher.bedrock')}
      </div>
      {isLoading ? (
        <Skeleton className="h-12 w-72 rounded-xl" />
      ) : !data?.supported ? (
        <div className="flex flex-col items-center gap-2" data-testid="bedrock-unsupported">
          <Badge variant="neutral" size="md">
            {t('bedrock.windowsOnly')}
          </Badge>
          <p className="max-w-md text-sm text-white/75">{t('bedrock.windowsOnlyHint')}</p>
        </div>
      ) : data.installed ? (
        <div className="flex flex-col items-center gap-3" data-testid="bedrock-installed">
          <Button
            variant="primary"
            size="xl"
            leftIcon={<Play className="fill-current" />}
            loading={pending}
            onClick={() => void act('bedrock_launch')}
            data-testid="bedrock-play"
          >
            {t('bedrock.play')}
          </Button>
          <p className="text-xs text-white/70">
            {data.version
              ? t('bedrock.installedVersion', { version: data.version })
              : t('bedrock.installed')}
          </p>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-3" data-testid="bedrock-missing">
          <p className="text-sm text-white/80">{t('bedrock.missing')}</p>
          <Button
            variant="primary"
            size="lg"
            rightIcon={<ExternalLink />}
            loading={pending}
            onClick={() => void act('bedrock_open_store')}
            data-testid="bedrock-store"
          >
            {t('bedrock.store')}
          </Button>
        </div>
      )}
      <p className="max-w-lg text-[11px] text-white/55">{t('bedrock.note')}</p>
      {data?.supported && (
        <Button
          size="sm"
          variant="ghost"
          className="absolute end-3 bottom-3 bg-black/30 text-white/80"
          leftIcon={<RefreshCw />}
          loading={isFetching}
          onClick={() => void refetch()}
        >
          {t('bedrock.refresh')}
        </Button>
      )}
    </section>
  );
}
