import { cn, Tabs, TabsContent, TabsList, TabsTrigger } from '@rustic/ui';
import type { ReactNode } from 'react';

export interface CatalogTab<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
  /** A count shown in a pill next to the label. */
  count?: number;
}

/**
 * The tab header of a catalogue page (Discover / Browse / Mine, the Servers and Profiles pages
 * next): a controlled Radix tab set (roles, arrow keys, Home/End) in the app's pill style. Put the
 * header, any toolbar, then one {@link CatalogTabPanel} per tab inside.
 */
export function CatalogTabs<T extends string>({
  value,
  onValueChange,
  children,
  className,
}: {
  value: T;
  onValueChange: (v: T) => void;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Tabs
      variant="pill"
      value={value}
      onValueChange={(v) => onValueChange(v as T)}
      className={cn('gap-4', className)}
    >
      {children}
    </Tabs>
  );
}

export function CatalogTabList<T extends string>({
  tabs,
  label,
  className,
  testIdPrefix,
}: {
  tabs: CatalogTab<T>[];
  /** Accessible name of the tab list. */
  label: string;
  className?: string;
  /** `data-testid` of each tab is `<prefix>-<value>`. */
  testIdPrefix?: string;
}) {
  return (
    <TabsList
      aria-label={label}
      className={cn(
        'bg-surface-2 isolate h-11 rounded-xl p-1 [&>button]:h-9 [&>button]:px-4',
        className,
      )}
    >
      {tabs.map((tab) => (
        <TabsTrigger
          key={tab.value}
          value={tab.value}
          icon={tab.icon}
          count={tab.count}
          data-testid={testIdPrefix ? `${testIdPrefix}-${tab.value}` : undefined}
          className="rounded-lg font-semibold"
        >
          {tab.label}
        </TabsTrigger>
      ))}
    </TabsList>
  );
}

export function CatalogTabPanel({
  value,
  children,
  className,
}: {
  value: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <TabsContent value={value} className={cn('mt-0 flex flex-col gap-6', className)}>
      {children}
    </TabsContent>
  );
}
