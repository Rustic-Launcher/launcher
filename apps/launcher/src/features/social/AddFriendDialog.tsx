/**
 * "Add a friend" dialog (ADR 0145): find a player by their exact Rustic or Minecraft name, see
 * who it is before sending, and pick from players met recently (same party or shared server).
 * One host per window (`AddFriendHost`): the launcher shell and the detachable Social window.
 */
import {
  Avatar,
  Badge,
  Button,
  Dialog,
  Input,
  Label,
  SegmentedControl,
  Skeleton,
} from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import {
  AtSign,
  Check,
  Clock,
  Gamepad2,
  Search,
  ShieldCheck,
  Swords,
  UserPlus,
} from 'lucide-react';
import { useId, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { errorCode } from '@/cloud/errors';
import type { FriendSuggestion, FriendTarget, UserLite, UserLookup } from '@/cloud/types';
import { cloud } from '@/cloud/client';
import { formatRelative } from '@/lib/format';
import {
  sk,
  useFriendRequestActions,
  useFriendRequests,
  useFriendSuggestions,
  useMe,
} from '@/queries/social';
import { useUi } from '@/stores/ui';
import { friendErrorKey, validName, type FriendErrorKey, type NameKind } from './addFriend';

/** Per-user outcome shared by the result card and the suggestion rows. */
type Outcome = { state: 'sent' } | { state: 'friends' } | { state: 'error'; code: string | null };

export function AddFriendHost() {
  const open = useUi((s) => s.addFriendOpen);
  const setOpen = useUi((s) => s.setAddFriendOpen);
  return <AddFriendDialog open={open} onOpenChange={setOpen} />;
}

export function AddFriendDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      size="lg"
      icon={<UserPlus />}
      title={t('addFriend.title')}
      description={t('addFriend.description')}
      closeLabel={t('actions.close')}
    >
      <AddFriendBody />
    </Dialog>
  );
}

function AddFriendBody() {
  const qc = useQueryClient();
  const [outcomes, setOutcomes] = useState<Record<string, Outcome>>({});
  const { send } = useFriendRequestActions();
  const [pending, setPending] = useState<ReadonlySet<string>>(new Set());

  const setOutcome = (id: string, o: Outcome) => setOutcomes((m) => ({ ...m, [id]: o }));

  /** Sends a request; the outcome is shown on every row of that user (several can run at once). */
  const sendTo = async (user: UserLite, target: FriendTarget) => {
    setPending((p) => new Set(p).add(user.id));
    try {
      await send.mutateAsync(target);
      // Inline "Request sent" (role=status) rather than a toast: a toast would be the top layer
      // and take the first Escape away from the dialog.
      setOutcome(user.id, { state: 'sent' });
    } catch (e) {
      setOutcome(user.id, { state: 'error', code: errorCode(e) });
    } finally {
      setPending((p) => {
        const next = new Set(p);
        next.delete(user.id);
        return next;
      });
    }
  };

  return (
    <div className="flex flex-col gap-5 pb-1">
      <LookupForm
        outcomes={outcomes}
        pending={pending}
        onSend={sendTo}
        onAccepted={(id) => {
          setOutcome(id, { state: 'friends' });
          void qc.invalidateQueries({ queryKey: sk.friends });
        }}
      />
      <Suggestions
        outcomes={outcomes}
        pending={pending}
        onSend={sendTo}
        onAccepted={(id) => setOutcome(id, { state: 'friends' })}
      />
    </div>
  );
}

function LookupForm({
  outcomes,
  pending,
  onSend,
  onAccepted,
}: {
  outcomes: Record<string, Outcome>;
  pending: ReadonlySet<string>;
  onSend: (user: UserLite, target: FriendTarget) => Promise<void>;
  onAccepted: (userId: string) => void;
}) {
  const { t } = useTranslation();
  const inputId = useId();
  const hintId = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const [kind, setKind] = useState<NameKind>('rustic');
  const [name, setName] = useState('');
  const [result, setResult] = useState<UserLookup | null>(null);
  const [error, setError] = useState<FriendErrorKey | null>(null);
  const [busy, setBusy] = useState(false);

  const lookup = async (e: FormEvent) => {
    e.preventDefault();
    const n = name.trim();
    setResult(null);
    if (!validName(kind, n)) {
      setError(friendErrorKey('invalid_input', kind));
      return;
    }
    setError(null);
    setBusy(true);
    try {
      setResult(await (await cloud()).lookupUser(kind, n));
    } catch (err) {
      setError(friendErrorKey(errorCode(err), kind));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="flex flex-col gap-3">
      <form onSubmit={(e) => void lookup(e)} className="flex flex-col gap-2" noValidate>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <Label htmlFor={inputId}>
            {kind === 'rustic' ? t('addFriend.rusticLabel') : t('addFriend.minecraftLabel')}
          </Label>
          <SegmentedControl
            size="sm"
            label={t('addFriend.kind')}
            value={kind}
            onValueChange={(v) => {
              setKind(v as NameKind);
              setError(null);
              setResult(null);
              inputRef.current?.focus();
            }}
            options={[
              { value: 'rustic', label: t('addFriend.kindRustic'), icon: <AtSign /> },
              { value: 'minecraft', label: t('addFriend.kindMinecraft'), icon: <Gamepad2 /> },
            ]}
          />
        </div>
        <div className="flex gap-2">
          <Input
            ref={inputRef}
            id={inputId}
            data-autofocus
            autoComplete="off"
            spellCheck={false}
            maxLength={kind === 'rustic' ? 20 : 16}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              if (error) setError(null);
            }}
            placeholder={
              kind === 'rustic'
                ? t('addFriend.rusticPlaceholder')
                : t('addFriend.minecraftPlaceholder')
            }
            leftIcon={kind === 'rustic' ? <AtSign /> : <Gamepad2 />}
            invalid={!!error}
            aria-describedby={hintId}
            className="flex-1"
            data-testid="add-friend-input"
          />
          <Button
            type="submit"
            leftIcon={<Search />}
            loading={busy}
            disabled={!name.trim()}
            data-testid="add-friend-find"
          >
            {t('addFriend.find')}
          </Button>
        </div>
        {error ? (
          <p
            id={hintId}
            role="alert"
            className="text-danger-text text-xs"
            data-testid="add-friend-error"
          >
            {t(`addFriend.errors.${error}`)}
          </p>
        ) : (
          <p id={hintId} className="text-fg-subtle text-xs">
            {kind === 'rustic' ? t('addFriend.rusticHint') : t('addFriend.minecraftHint')}
          </p>
        )}
      </form>
      {busy && <Skeleton className="h-16 rounded-xl" />}
      {result && (
        <ResultCard
          result={result}
          outcome={outcomes[result.user.id]}
          sending={pending.has(result.user.id)}
          onSend={() => void onSend(result.user, { user_id: result.user.id })}
          onAccepted={() => onAccepted(result.user.id)}
          kind={kind}
        />
      )}
    </section>
  );
}

function ResultCard({
  result,
  outcome,
  sending,
  onSend,
  onAccepted,
  kind,
}: {
  result: UserLookup;
  outcome: Outcome | undefined;
  sending: boolean;
  onSend: () => void;
  onAccepted: () => void;
  kind: NameKind;
}) {
  const { t } = useTranslation();
  const { respond } = useFriendRequestActions();
  const u = result.user;
  const relation = outcome?.state === 'friends' ? 'friends' : result.relation;

  let action: ReactNode;
  if (outcome?.state === 'sent') action = <SentLabel />;
  else if (relation === 'friends')
    action = <Badge variant="success">{t('addFriend.friends')}</Badge>;
  else if (relation === 'outgoing_request')
    action = <Badge variant="neutral">{t('addFriend.pending')}</Badge>;
  else if (relation === 'incoming_request' && result.request_id) {
    const id = result.request_id;
    action = (
      <Button
        size="sm"
        variant="primary"
        leftIcon={<Check />}
        loading={respond.isPending}
        onClick={() => respond.mutate({ id, accept: true }, { onSuccess: () => onAccepted() })}
      >
        {t('friends.accept')}
      </Button>
    );
  } else
    action = (
      <Button
        size="sm"
        variant="primary"
        leftIcon={<UserPlus />}
        loading={sending}
        onClick={onSend}
        data-testid="add-friend-send"
      >
        {t('addFriend.send')}
      </Button>
    );

  return (
    <div
      className="border-border bg-surface-2/60 flex flex-col gap-2 rounded-xl border p-3"
      data-testid="add-friend-result"
      aria-live="polite"
    >
      <div className="flex items-center gap-3">
        <Avatar name={u.display_name} size="md" />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm font-semibold">{u.display_name}</span>
          <span className="text-fg-subtle truncate text-xs">@{u.username}</span>
          {result.minecraft_name && (
            <span className="text-fg-muted mt-0.5 flex items-center gap-1 truncate text-xs">
              <Gamepad2 className="size-3.5 shrink-0" aria-hidden />
              <span className="sr-only">{t('addFriend.minecraftName')}</span>
              {result.minecraft_name}
            </span>
          )}
        </div>
        {action}
      </div>
      {relation === 'incoming_request' && outcome?.state !== 'sent' && (
        <p className="text-fg-muted text-xs">{t('addFriend.incoming')}</p>
      )}
      {outcome?.state === 'error' && <RowError code={outcome.code} kind={kind} />}
    </div>
  );
}

function SentLabel() {
  const { t } = useTranslation();
  return (
    <span
      role="status"
      className="text-success-text flex shrink-0 items-center gap-1 text-xs font-medium"
    >
      <Check className="size-3.5" aria-hidden />
      {t('addFriend.sent')}
    </span>
  );
}

function RowError({ code, kind }: { code: string | null; kind: NameKind }) {
  const { t } = useTranslation();
  return (
    <p role="alert" className="text-danger-text text-xs">
      {t(`addFriend.errors.${friendErrorKey(code, kind)}`)}
    </p>
  );
}

function Suggestions({
  outcomes,
  pending,
  onSend,
  onAccepted,
}: {
  outcomes: Record<string, Outcome>;
  pending: ReadonlySet<string>;
  onSend: (user: UserLite, target: FriendTarget) => Promise<void>;
  onAccepted: (userId: string) => void;
}) {
  const { t } = useTranslation();
  const { data, isLoading, isError, refetch } = useFriendSuggestions();
  const { data: me } = useMe();

  if (isError) {
    return (
      <p className="text-fg-muted text-sm">
        {t('addFriend.suggestionsError')}{' '}
        <Button variant="link" size="sm" onClick={() => void refetch()}>
          {t('actions.retry')}
        </Button>
      </p>
    );
  }
  if (data && !data.discoverable) {
    return (
      <p
        className="border-border text-fg-muted flex items-start gap-2 rounded-xl border border-dashed p-3 text-sm"
        data-testid="suggestions-off"
      >
        <ShieldCheck className="mt-0.5 size-4 shrink-0" aria-hidden />
        {me && !me.discoverable_allowed
          ? t('addFriend.suggestionsMinor')
          : t('addFriend.suggestionsOff')}
      </p>
    );
  }
  const section = (
    id: string,
    title: string,
    icon: ReactNode,
    list: FriendSuggestion[] | undefined,
    empty: string,
  ) => (
    <section aria-labelledby={id} data-testid={`suggestions-${id}`}>
      <h3
        id={id}
        className="text-2xs text-fg-subtle flex items-center gap-1.5 pb-1 font-semibold tracking-wider uppercase [&_svg]:size-3.5"
      >
        {icon}
        {title}
      </h3>
      {isLoading || !list ? (
        <div className="flex flex-col gap-2 pt-1" aria-busy="true">
          {Array.from({ length: 3 }, (_, i) => (
            <Skeleton key={i} className="h-11" />
          ))}
        </div>
      ) : list.length === 0 ? (
        <p className="text-fg-subtle py-2 text-sm">{empty}</p>
      ) : (
        <ul className="-mx-2 flex flex-col">
          {list.map((s) => (
            <SuggestionRow
              key={s.user.id}
              s={s}
              outcome={outcomes[s.user.id]}
              sending={pending.has(s.user.id)}
              onSend={() => void onSend(s.user, { user_id: s.user.id })}
              onAccepted={() => onAccepted(s.user.id)}
            />
          ))}
        </ul>
      )}
    </section>
  );

  return (
    <div className="border-border flex flex-col gap-4 border-t pt-4">
      {section(
        'played-with',
        t('addFriend.playedWith'),
        <Swords aria-hidden />,
        data?.played_with,
        t('addFriend.playedWithEmpty'),
      )}
      {section(
        'crossed',
        t('addFriend.crossed'),
        <Clock aria-hidden />,
        data?.crossed,
        t('addFriend.crossedEmpty'),
      )}
      <p className="text-fg-subtle flex items-start gap-1.5 text-xs">
        <ShieldCheck className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('addFriend.privacyNote')}
      </p>
    </div>
  );
}

function SuggestionRow({
  s,
  outcome,
  sending,
  onSend,
  onAccepted,
}: {
  s: FriendSuggestion;
  outcome: Outcome | undefined;
  sending: boolean;
  onSend: () => void;
  onAccepted: () => void;
}) {
  const { t, i18n } = useTranslation();
  const { respond } = useFriendRequestActions();
  const { data: requests } = useFriendRequests();
  const u = s.user;
  const why =
    s.reason === 'party'
      ? t('addFriend.reasonParty')
      : s.server_name
        ? t('addFriend.reasonServer', { server: s.server_name })
        : t('addFriend.reasonSameServer');
  // They sent us a request meanwhile: offer to accept it.
  const incoming =
    outcome?.state === 'error' && outcome.code === 'incoming_request_exists'
      ? requests?.find((r) => r.direction === 'incoming' && r.user.id === u.id)
      : undefined;

  return (
    <li
      className="hover:bg-surface-2 flex flex-col gap-1 rounded-lg px-2 py-1.5"
      data-testid={`suggestion-${u.username}`}
    >
      <div className="flex items-center gap-2.5">
        <Avatar name={u.display_name} size="sm" />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm font-medium">
            {u.display_name} <span className="text-fg-subtle font-normal">@{u.username}</span>
          </span>
          <span className="text-fg-subtle truncate text-xs">
            {why} · {formatRelative(s.seen_at, i18n.language)}
            {s.minecraft_name && (
              <>
                {' · '}
                <Gamepad2 className="inline size-3 align-[-2px]" aria-hidden />{' '}
                <span className="sr-only">{t('addFriend.minecraftName')}</span>
                {s.minecraft_name}
              </>
            )}
          </span>
        </div>
        {outcome?.state === 'sent' ? (
          <SentLabel />
        ) : outcome?.state === 'friends' ? (
          <Badge variant="success">{t('addFriend.friends')}</Badge>
        ) : incoming ? (
          <Button
            size="sm"
            variant="primary"
            leftIcon={<Check />}
            loading={respond.isPending}
            onClick={() =>
              respond.mutate({ id: incoming.id, accept: true }, { onSuccess: onAccepted })
            }
          >
            {t('friends.accept')}
          </Button>
        ) : (
          <Button
            size="sm"
            variant="soft"
            leftIcon={<UserPlus />}
            loading={sending}
            onClick={onSend}
            aria-label={t('addFriend.addAria', { name: u.display_name })}
          >
            {t('addFriend.add')}
          </Button>
        )}
      </div>
      {outcome?.state === 'error' && <RowError code={outcome.code} kind="rustic" />}
    </li>
  );
}
