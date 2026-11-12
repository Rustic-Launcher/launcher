import {
  Gamepad2,
  House,
  Puzzle,
  Server,
  Settings,
  Shirt,
  ShoppingBag,
  type LucideIcon,
} from 'lucide-react';
import { STORE_URL } from '@/features/settings/legal';

export interface NavItem {
  id: 'home' | 'instances' | 'content' | 'servers' | 'locker' | 'store' | 'settings';
  /** A page of the launcher… */
  to?: '/' | '/instances' | '/content' | '/servers' | '/locker' | '/settings';
  /** …or a page of the website, opened in the browser. */
  href?: string;
  icon: LucideIcon;
  /** Keyboard shortcut (Alt + digit). */
  shortcut?: string;
}

export const MAIN_NAV: NavItem[] = [
  { id: 'home', to: '/', icon: House, shortcut: 'Alt+1' },
  { id: 'instances', to: '/instances', icon: Gamepad2, shortcut: 'Alt+2' },
  { id: 'content', to: '/content', icon: Puzzle, shortcut: 'Alt+3' },
  { id: 'servers', to: '/servers', icon: Server, shortcut: 'Alt+4' },
  { id: 'locker', to: '/locker', icon: Shirt, shortcut: 'Alt+5' },
  { id: 'store', href: STORE_URL, icon: ShoppingBag, shortcut: 'Alt+6' },
];

export const SETTINGS_NAV: NavItem = {
  id: 'settings',
  to: '/settings',
  icon: Settings,
  shortcut: 'Alt+,',
};

export function isActive(pathname: string, to: NavItem['to']): boolean {
  if (!to) return false;
  if (to === '/') return pathname === '/';
  return pathname === to || pathname.startsWith(`${to}/`);
}
