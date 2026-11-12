import { cn, Tooltip } from '@rustic/ui';
import { Link, useNavigate, useRouterState } from '@tanstack/react-router';
import { motion } from 'motion/react';
import { useEffect, useRef, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { SquarePlus } from 'lucide-react';
import { invoke } from '@/ipc';
import { isActive, MAIN_NAV, SETTINGS_NAV, type NavItem } from './nav';

/**
 * Left vertical icon rail. Keyboard: Tab enters the rail, Up/Down/Home/End move between items
 * (roving focus), Enter activates. Alt+1..6 and Alt+, jump from anywhere.
 */
export function Rail() {
  const { t } = useTranslation();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const navigate = useNavigate();
  const listRef = useRef<HTMLElement>(null);
  const all = [...MAIN_NAV, SETTINGS_NAV];

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (!e.altKey || e.ctrlKey || e.metaKey) return;
      const idx = Number(e.key) - 1;
      const item = e.key === ',' ? SETTINGS_NAV : MAIN_NAV[idx];
      if (item) {
        e.preventDefault();
        if (item.href) void invoke('app_open_url', { url: item.href });
        else void navigate({ to: item.to ?? '/' });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [navigate]);

  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    const links = Array.from(
      listRef.current?.querySelectorAll<HTMLAnchorElement>('a[data-rail]') ?? [],
    );
    const i = links.indexOf(document.activeElement as HTMLAnchorElement);
    if (i < 0) return;
    let next = -1;
    if (e.key === 'ArrowDown') next = (i + 1) % links.length;
    else if (e.key === 'ArrowUp') next = (i - 1 + links.length) % links.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = links.length - 1;
    if (next >= 0) {
      e.preventDefault();
      links[next]?.focus();
    }
  };

  const activeIndex = all.findIndex((n) => isActive(pathname, n.to));

  const item = (n: NavItem, i: number) => {
    const active = isActive(pathname, n.to);
    const Icon = n.icon;
    // Roving tabindex: only the active item (or the first) is in the tab order.
    const tabbable = activeIndex >= 0 ? i === activeIndex : i === 0;
    return (
      <li key={n.id} className="relative flex justify-center">
        {active && (
          <motion.span
            layoutId="rail-active"
            className="bg-accent absolute top-1/2 left-0 h-6 w-1 -translate-y-1/2 rounded-r-full"
            transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
          />
        )}
        <Tooltip content={t(`nav.${n.id}`)} side="right" shortcut={n.shortcut}>
          {n.href ? (
            // A website page: opened in the browser, never inside the launcher.
            <a
              href={n.href}
              target="_blank"
              rel="noreferrer"
              onClick={(e) => {
                e.preventDefault();
                void invoke('app_open_url', { url: n.href! });
              }}
              data-rail
              data-testid={`rail-${n.id}`}
              tabIndex={tabbable ? 0 : -1}
              aria-label={t(`nav.${n.id}`)}
              className="rx-focus-ring text-fg-subtle hover:bg-surface-2 hover:text-fg grid size-11 place-items-center rounded-xl transition-colors duration-150"
            >
              <Icon className="size-[22px]" strokeWidth={1.8} />
            </a>
          ) : (
            <Link
              to={n.to ?? '/'}
              data-rail
              data-testid={`rail-${n.id}`}
              tabIndex={tabbable ? 0 : -1}
              aria-label={t(`nav.${n.id}`)}
              aria-current={active ? 'page' : undefined}
              className={cn(
                'rx-focus-ring grid size-11 place-items-center rounded-xl transition-colors duration-150',
                active ? 'text-fg' : 'text-fg-subtle hover:bg-surface-2 hover:text-fg',
              )}
            >
              <Icon className="size-[22px]" strokeWidth={active ? 2.2 : 1.8} />
            </Link>
          )}
        </Tooltip>
      </li>
    );
  };

  return (
    <nav
      ref={listRef}
      aria-label={t('nav.label')}
      onKeyDown={onKeyDown}
      className="z-10 col-start-1 row-start-2 flex flex-col items-center pt-2 pb-3"
    >
      <ul className="flex flex-1 flex-col gap-2">{MAIN_NAV.map((n, i) => item(n, i))}</ul>
      <div className="bg-border-default mb-3 h-px w-8" role="separator" />
      <ul className="flex flex-col gap-2">
        <li className="flex justify-center">
          <Tooltip content={t('nav.newInstance')} side="right">
            <Link
              to="/instances"
              search={{ new: '1' }}
              data-testid="rail-new"
              tabIndex={-1}
              aria-label={t('nav.newInstance')}
              className="rx-focus-ring text-fg-subtle hover:bg-surface-2 hover:text-fg grid size-11 place-items-center rounded-xl transition-colors"
            >
              <SquarePlus className="size-[22px]" strokeWidth={1.8} />
            </Link>
          </Tooltip>
        </li>
        {item(SETTINGS_NAV, MAIN_NAV.length)}
      </ul>
    </nav>
  );
}
