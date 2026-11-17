/**
 * The Party tab of the Social view (ADR 0176): invites received, the party (members, leader
 * actions, voice and its proximity setting), and the party chat in the right pane.
 */
import {
  Avatar,
  Button,
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownTrigger,
  IconButton,
  Skeleton,
  Slider,
  Switch,
  toast,
} from '@rustic/ui';
import {
  Check,
  Crown,
  Flag,
  MoreHorizontal,
  PartyPopper,
  SendHorizontal,
  UserMinus,
  Users,
  X,
} from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import { errorCode } from '@/cloud/errors';
import type { Party } from '@/cloud/types';
import { formatRelative } from '@/lib/format';
import {
  useMe,
  useParty,
  usePartyActions,
  usePartyInvites,
  usePartyMessages,
  useSendPartyMessage,
} from '@/queries/social';
import { VoicePanel } from '@/features/voice/VoiceControls';
import { useVoice } from '@/features/voice/store';
import {
  clampRange,
  DEFAULT_RANGE,
  fullVolumeWithin,
  MAX_RANGE,
  MIN_RANGE,
} from '@/features/voice/proximity';

function useErrorToast() {
  const { t } = useTranslation();
  return (e: unknown) => {
    const known = {
      party_full: t('party.errors.party_full'),
      party_not_allowed: t('party.errors.party_not_allowed'),
      already_in_party: t('party.errors.already_in_party'),
      not_party_leader: t('party.errors.not_party_leader'),
    } as Record<string, string>;
    toast({ tone: 'error', title: known[errorCode(e) ?? ''] ?? t('errors.generic') });
  };
}

function Invites() {
  const { t } = useTranslation();
  const { data: invites } = usePartyInvites();
  const { respond } = usePartyActions();
  const onError = useErrorToast();
  if (!invites?.length) return null;
  return (
    <section className="flex flex-col gap-1.5" aria-label={t('party.invitesTitle')}>
      <h3 className="text-fg-subtle px-1 text-xs font-semibold">{t('party.invitesTitle')}</h3>
      {invites.map((i) => (
        <div
          key={i.id}
          className="rx-surface flex items-center gap-2 rounded-xl p-2"
          data-testid={`party-invite-${i.from.username}`}
        >
          <Avatar name={i.from.display_name} src={i.from.avatar_url ?? undefined} size="sm" />
          <span className="min-w-0 flex-1 truncate text-sm">
            {t('party.invitedBy', { name: i.from.display_name })}
          </span>
          <IconButton
            size="sm"
            variant="primary"
            label={t('party.accept')}
            icon={<Check />}
            onClick={() => respond.mutate({ id: i.id, accept: true }, { onError })}
          />
          <IconButton
            size="sm"
            label={t('party.decline')}
            icon={<X />}
            onClick={() => respond.mutate({ id: i.id, accept: false }, { onError })}
          />
        </div>
      ))}
    </section>
  );
}

function MemberRow({
  party,
  member,
  meId,
}: {
  party: Party;
  member: Party['members'][number];
  meId?: string;
}) {
  const { t } = useTranslation();
  const { kick, transfer } = usePartyActions();
  const onError = useErrorToast();
  const speaking = useVoice(
    (s) => s.participants.find((p) => p.id === member.id)?.speaking ?? false,
  );
  const selfSpeaking = useVoice((s) => s.selfSpeaking);
  const isMe = member.id === meId;
  const leader = party.leader_id === meId;
  const talking = isMe ? selfSpeaking : speaking;
  return (
    <li
      className="group hover:bg-surface-2 flex items-center gap-2.5 rounded-lg px-2 py-1.5"
      data-testid={`party-member-${member.username}`}
    >
      <span
        className={cn(
          'relative rounded-full ring-2 ring-offset-1 ring-offset-transparent',
          talking ? 'ring-success' : 'ring-transparent',
        )}
      >
        <Avatar
          name={member.display_name}
          src={member.avatar_url ?? undefined}
          size="sm"
          presence={member.presence}
          presenceLabel={t(`presence.${member.presence}`)}
        />
      </span>
      <span className="flex min-w-0 flex-1 items-center gap-1 text-sm font-medium">
        <span className="truncate">{member.display_name}</span>
        {isMe && <span className="text-fg-subtle text-xs font-normal">({t('voice.you')})</span>}
        {member.id === party.leader_id && (
          <Crown className="text-warning size-3.5 shrink-0" aria-label={t('party.leader')} />
        )}
      </span>
      {leader && !isMe && (
        <Dropdown>
          <DropdownTrigger asChild>
            <IconButton
              size="sm"
              tooltip={false}
              label={t('friends.more', { name: member.display_name })}
              icon={<MoreHorizontal />}
            />
          </DropdownTrigger>
          <DropdownContent>
            <DropdownItem icon={<Crown />} onSelect={() => transfer.mutate(member.id, { onError })}>
              {t('party.makeLeader')}
            </DropdownItem>
            <DropdownItem
              icon={<UserMinus />}
              destructive
              onSelect={() => kick.mutate(member.id, { onError })}
            >
              {t('party.kick')}
            </DropdownItem>
          </DropdownContent>
        </Dropdown>
      )}
    </li>
  );
}

function ProximitySetting({ party, leader }: { party: Party; leader: boolean }) {
  const { t } = useTranslation();
  const { voice } = usePartyActions();
  const onError = useErrorToast();
  const [range, setRange] = useState(party.voice.range);
  useEffect(() => setRange(party.voice.range), [party.voice.range]);
  return (
    <div className="flex flex-col gap-2 px-1" data-testid="party-proximity">
      <label className="flex items-center justify-between gap-3 text-sm">
        <span className="flex flex-col">
          <span className="font-medium">{t('party.proximity')}</span>
          <span className="text-fg-subtle text-xs">{t('party.proximityHint')}</span>
        </span>
        <Switch
          checked={party.voice.proximity}
          disabled={!leader || voice.isPending}
          aria-label={t('party.proximity')}
          onCheckedChange={(on) => voice.mutate({ proximity: on }, { onError })}
        />
      </label>
      {party.voice.proximity && (
        <div className="flex flex-col gap-1.5" data-testid="party-range">
          <div className="flex items-baseline justify-between gap-3 text-xs">
            <span className="text-fg-muted font-medium">{t('party.hearing')}</span>
            <span className="font-semibold text-white tabular-nums" data-testid="party-range-value">
              {t('party.rangeBlocks', { count: range })}
            </span>
          </div>
          <Slider
            label={t('party.hearing')}
            value={range}
            min={MIN_RANGE}
            max={MAX_RANGE}
            step={4}
            disabled={!leader}
            formatValue={(v) => t('party.rangeBlocks', { count: v })}
            marks={[MIN_RANGE, DEFAULT_RANGE, MAX_RANGE].map((value) => ({
              value,
              label: String(value),
            }))}
            onValueChange={setRange}
            onValueCommit={(v) => voice.mutate({ range: clampRange(v) }, { onError })}
          />
          {/* The real curve (proximityGain): full volume within 15 % of the range, then a fade. */}
          <p className="text-fg-subtle text-xs">
            {t('party.rangeHelp', {
              near: t('party.rangeBlocks', { count: fullVolumeWithin(range) }),
              far: t('party.rangeBlocks', { count: range }),
            })}
          </p>
        </div>
      )}
    </div>
  );
}

/** The left column of the Party tab. */
export function PartyTab() {
  const { t } = useTranslation();
  const { data: party, isLoading } = useParty();
  const { data: me } = useMe();
  const { create, leave } = usePartyActions();
  const onError = useErrorToast();
  if (isLoading) return <Skeleton className="h-40" />;
  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto" data-testid="party-tab">
      <Invites />
      {!party ? (
        <div className="flex flex-col items-center gap-3 px-4 py-8 text-center">
          <span className="bg-surface-3 text-fg-muted grid size-14 place-items-center rounded-full">
            <PartyPopper className="size-6" />
          </span>
          <p className="text-sm font-semibold">{t('party.emptyTitle')}</p>
          <p className="text-fg-muted text-xs">{t('party.emptyBody')}</p>
          <Button
            variant="primary"
            data-testid="party-create"
            loading={create.isPending}
            onClick={() => create.mutate(undefined, { onError })}
          >
            {t('party.create')}
          </Button>
        </div>
      ) : (
        <>
          <section
            className="flex flex-col gap-1"
            aria-label={t('party.title')}
            data-testid="party-members"
          >
            <div className="flex items-center justify-between px-1">
              <h3 className="flex items-center gap-1.5 text-xs font-semibold">
                <Users className="text-accent-text size-3.5" aria-hidden />
                {t('party.title')}
                <span className="text-fg-subtle font-normal">
                  {party.members.length}/{party.max_size}
                </span>
              </h3>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2"
                data-testid="party-leave"
                onClick={() => leave.mutate(undefined, { onError })}
              >
                {t('party.leave')}
              </Button>
            </div>
            <ul className="-mx-1 flex flex-col">
              {party.members.map((m) => (
                <MemberRow key={m.id} party={party} member={m} meId={me?.id} />
              ))}
              {party.pending_invites.map((u) => (
                <li key={u.id} className="flex items-center gap-2.5 px-2 py-1.5 opacity-60">
                  <Avatar name={u.display_name} size="sm" />
                  <span className="truncate text-sm">
                    {t('party.pending', { name: u.display_name })}
                  </span>
                </li>
              ))}
            </ul>
            <p className="text-fg-subtle px-1 text-xs">{t('party.inviteHint')}</p>
          </section>
          <VoicePanel kind="party" allowed={me?.can_party_voice ?? true} />
          <ProximitySetting party={party} leader={party.leader_id === me?.id} />
        </>
      )}
    </div>
  );
}

/** The right pane of the Party tab: the party chat. */
export function PartyChatPane() {
  const { t, i18n } = useTranslation();
  const { data: party } = useParty();
  const { data: me } = useMe();
  const { data: messages, isLoading } = usePartyMessages(!!party);
  const send = useSendPartyMessage();
  const onError = useErrorToast();
  const [draft, setDraft] = useState('');
  const [reported, setReported] = useState<Set<string>>(new Set());
  const endRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    endRef.current?.scrollIntoView({ block: 'end' });
  }, [messages?.length]);

  if (!party) {
    return (
      <div
        className="flex h-full flex-col items-center justify-center gap-3 text-center"
        data-testid="party-chat-empty"
      >
        <span className="bg-surface-3 text-fg-muted grid size-16 place-items-center rounded-full">
          <PartyPopper className="size-7" />
        </span>
        <p className="text-lg font-semibold">{t('party.chatTitle')}</p>
        <p className="text-fg-muted max-w-64 text-sm">{t('party.chatEmpty')}</p>
      </div>
    );
  }
  const name = (id: string) =>
    party.members.find((m) => m.id === id)?.display_name ?? t('party.formerMember');
  const report = async (messageId: string, userId: string) => {
    try {
      await (
        await cloud()
      ).reportUser({
        user_id: userId,
        reason: 'harassment',
        context: 'party_chat',
        party_message_id: messageId,
      });
      setReported((s) => new Set(s).add(messageId));
      toast({ tone: 'success', title: t('voice.reported', { name: name(userId) }) });
    } catch (e) {
      onError(e);
    }
  };
  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="party-chat">
      <header className="border-border flex items-center gap-3 border-b px-4 py-3">
        <PartyPopper className="text-accent-text size-4" aria-hidden />
        <span className="text-sm font-semibold">{t('party.chatTitle')}</span>
      </header>
      <div
        className="min-h-0 flex-1 overflow-y-auto px-4 py-3"
        role="log"
        aria-label={t('party.chatTitle')}
      >
        {isLoading && <Skeleton className="h-10 w-1/2" />}
        <ul className="flex flex-col gap-1.5">
          {messages?.map((m) => {
            const mine = m.from_id === me?.id;
            return (
              <li
                key={m.id}
                className={cn('group flex flex-col', mine ? 'items-end' : 'items-start')}
              >
                {!mine && (
                  <span className="text-fg-subtle text-2xs mb-0.5 px-1">{name(m.from_id)}</span>
                )}
                <span className="flex items-center gap-1">
                  <span
                    className={cn(
                      'max-w-[75%] rounded-2xl px-3 py-2 text-sm',
                      mine
                        ? 'bg-accent text-accent-fg rounded-ee-md'
                        : 'bg-surface-3 text-fg rounded-es-md',
                    )}
                  >
                    {m.body}
                  </span>
                  {!mine && !reported.has(m.id) && (
                    <IconButton
                      size="sm"
                      label={t('voice.reportAria', { name: name(m.from_id) })}
                      icon={<Flag />}
                      className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
                      onClick={() => void report(m.id, m.from_id)}
                    />
                  )}
                </span>
                <span className="text-fg-subtle text-2xs mt-0.5">
                  {formatRelative(m.sent_at, i18n.language)}
                </span>
              </li>
            );
          })}
        </ul>
        <div ref={endRef} />
      </div>
      <form
        className="border-border flex items-center gap-2 border-t p-3"
        onSubmit={(e) => {
          e.preventDefault();
          const body = draft.trim();
          if (!body) return;
          send.mutate(body, { onSuccess: () => setDraft(''), onError });
        }}
      >
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          maxLength={1000}
          placeholder={t('party.chatPlaceholder')}
          aria-label={t('party.chatPlaceholder')}
          data-testid="party-chat-input"
          className="bg-surface-2 border-border-strong/40 text-fg placeholder:text-fg-subtle focus-visible:border-accent h-10 flex-1 rounded-lg border px-3 text-sm outline-none"
        />
        <IconButton
          type="submit"
          variant="primary"
          label={t('social.send')}
          icon={<SendHorizontal />}
          disabled={!draft.trim()}
        />
      </form>
    </div>
  );
}
