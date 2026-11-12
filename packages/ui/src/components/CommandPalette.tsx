import { Command } from 'cmdk';
import { Dialog as D } from 'radix-ui';
import { AnimatePresence, motion } from 'motion/react';
import { Search } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

export interface CommandItem {
  id: string;
  label: string;
  /** Extra words that should match (aliases, translations). */
  keywords?: string[];
  hint?: ReactNode;
  icon?: ReactNode;
  onSelect: () => void;
}

export interface CommandGroup {
  id: string;
  heading: string;
  items: CommandItem[];
}

export interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  groups: CommandGroup[];
  query: string;
  onQueryChange: (q: string) => void;
  placeholder: string;
  emptyText: string;
  title: string;
  loading?: boolean;
  loadingText?: string;
  footer?: ReactNode;
}

/** Global search / command palette (Ctrl+K). */
export function CommandPalette({
  open,
  onOpenChange,
  groups,
  query,
  onQueryChange,
  placeholder,
  emptyText,
  title,
  loading,
  loadingText,
  footer,
}: CommandPaletteProps) {
  return (
    <D.Root open={open} onOpenChange={onOpenChange}>
      <AnimatePresence>
        {open && (
          <D.Portal forceMount>
            <D.Overlay asChild forceMount>
              <motion.div
                className="bg-overlay fixed inset-0 z-[60]"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
              />
            </D.Overlay>
            <D.Content asChild forceMount aria-describedby={undefined}>
              <motion.div
                initial={{ opacity: 0, scale: 0.97, y: -8 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.98 }}
                transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
                className="rx-glass fixed top-[14vh] left-1/2 z-[60] w-[min(640px,92vw)] -translate-x-1/2 overflow-hidden rounded-2xl shadow-lg outline-none"
              >
                <D.Title className="sr-only">{title}</D.Title>
                <Command label={title} shouldFilter={true} loop>
                  <div className="border-border flex items-center gap-3 border-b px-4">
                    <Search className="text-fg-subtle size-4.5" aria-hidden />
                    <Command.Input
                      value={query}
                      onValueChange={onQueryChange}
                      placeholder={placeholder}
                      className="text-fg placeholder:text-fg-subtle h-14 flex-1 bg-transparent text-base outline-none"
                    />
                  </div>
                  <Command.List className="max-h-[50vh] overflow-y-auto p-2">
                    {loading && (
                      <Command.Loading>
                        <div className="text-fg-subtle px-3 py-2 text-sm">{loadingText}</div>
                      </Command.Loading>
                    )}
                    <Command.Empty className="text-fg-muted px-3 py-8 text-center text-sm">
                      {emptyText}
                    </Command.Empty>
                    {groups
                      .filter((g) => g.items.length > 0)
                      .map((g) => (
                        <Command.Group
                          key={g.id}
                          heading={g.heading}
                          className="[&_[cmdk-group-heading]]:text-2xs [&_[cmdk-group-heading]]:text-fg-subtle [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:pt-3 [&_[cmdk-group-heading]]:pb-1.5 [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:uppercase"
                        >
                          {g.items.map((it) => (
                            <Command.Item
                              key={it.id}
                              value={`${g.id}:${it.id} ${it.label}`}
                              keywords={it.keywords}
                              onSelect={() => {
                                it.onSelect();
                                onOpenChange(false);
                              }}
                              className={cn(
                                'text-fg data-[selected=true]:bg-surface-3 [&_svg]:text-fg-muted flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2.5 text-sm [&_svg]:size-4',
                              )}
                            >
                              {it.icon}
                              <span className="flex-1 truncate">{it.label}</span>
                              {it.hint && <span className="text-fg-subtle text-xs">{it.hint}</span>}
                            </Command.Item>
                          ))}
                        </Command.Group>
                      ))}
                  </Command.List>
                  {footer && (
                    <div className="border-border text-fg-subtle flex items-center gap-4 border-t px-4 py-2 text-xs">
                      {footer}
                    </div>
                  )}
                </Command>
              </motion.div>
            </D.Content>
          </D.Portal>
        )}
      </AnimatePresence>
    </D.Root>
  );
}
