import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function SettingsSortButton({
  className,
  ...props
}: ComponentProps<'button'>) {
  return (
    <button
      className={cn(
        'flex min-w-0 items-center gap-2 text-left max-md:min-h-11 [&_svg]:size-3 focus-visible:rounded-lg focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-4',
        className,
      )}
      {...props}
    />
  );
}

export function SettingsPerson({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      className={cn(
        'flex min-w-0 items-center gap-2.5 font-medium [&>span]:min-w-0 [&_small]:block [&_small]:text-xs [&_small]:font-normal [&_small]:text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}

export function WorkspaceAvatar({
  className,
  ...props
}: ComponentProps<'span'>) {
  return (
    <span
      className={cn(
        'inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-primary-foreground',
        className,
      )}
      {...props}
    />
  );
}
