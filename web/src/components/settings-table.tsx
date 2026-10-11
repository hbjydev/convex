import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function SettingsTable({
  className,
  kind,
  ...props
}: ComponentProps<'table'> & { kind: 'members' | 'teams' }) {
  return (
    <table
      className={cn(
        'w-full border-collapse text-left text-[13px] tabular-nums [&_th]:font-normal [&_th]:text-muted-foreground [&_tr:not([data-group])>th]:px-2.5 [&_tr:not([data-group])>th]:py-3 md:[&_tr:not([data-group])>th]:px-4 [&_td]:px-2.5 [&_td]:py-3 md:[&_td]:px-4 [&_td]:text-muted-foreground md:[&_th:first-child]:w-[44%] [&_tr:not([data-group])>td:first-child]:text-foreground [&_tbody_tr:not([data-group]):hover]:bg-muted/50 max-md:[&_tr]:grid max-md:[&_th]:min-w-0 max-md:[&_td]:min-w-0 max-md:[&_th]:whitespace-nowrap max-md:[&_td:not(:first-child)]:whitespace-nowrap max-md:[&_th]:wrap-normal max-md:[&_td:not(:first-child)]:wrap-normal',
        kind === 'members'
          ? 'max-md:[&_tr]:grid-cols-[minmax(0,1fr)_92px_44px] max-md:[&_th:nth-child(3)]:hidden max-md:[&_td:nth-child(3)]:hidden max-md:[&_[data-slot=button]]:min-w-11'
          : 'max-md:[&_tr]:grid-cols-[minmax(0,1fr)_80px_96px] max-md:[&_th:nth-child(4)]:hidden max-md:[&_td:nth-child(4)]:hidden',
        className,
      )}
      {...props}
    />
  );
}
