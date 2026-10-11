import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function SettingsPage({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      data-layout="settings"
      className={cn(
        'mx-auto w-full max-w-[640px] pt-2 md:pt-6 [&>h1]:mb-8 md:[&>h1]:mx-4 [&>h1]:text-2xl has-[>p]:[&>h1]:mb-2 [&>p]:mb-8 md:[&>p]:mx-4',
        className,
      )}
      {...props}
    />
  );
}

export function SettingsDirectory({
  className,
  ...props
}: ComponentProps<'div'>) {
  return (
    <div
      data-layout="directory"
      className={cn('pt-2 md:pt-6 [&>h1]:mb-4 [&>h1]:text-2xl', className)}
      {...props}
    />
  );
}

export function SettingsPanel({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div className={cn('rounded-xl bg-muted px-4', className)} {...props} />
  );
}

export function SettingsRow({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      className={cn(
        'flex min-h-16 flex-wrap items-center justify-between gap-3 py-4 text-[13px] md:flex-nowrap md:gap-6 [&+div]:border-t [&>input]:w-full md:[&>input]:w-64 md:[&>input]:max-w-1/2 [&>input]:shrink-0 [&>div[data-name-field]]:w-full md:[&>div[data-name-field]]:w-64 md:[&>div[data-name-field]]:max-w-1/2 [&>div[data-name-field]]:shrink-0 [&_p]:text-xs [&_p]:leading-[1.6] [&_p]:text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}

export function SettingsBlock({
  className,
  ...props
}: ComponentProps<'section'>) {
  return (
    <section
      className={cn(
        'mt-10 flex min-w-0 flex-col gap-3 in-[[data-layout=settings]]:md:[&>h2]:mx-4 in-[[data-layout=settings]]:md:[&>p]:mx-4 in-[[data-slot=dialog-content]]:mt-6',
        className,
      )}
      {...props}
    />
  );
}

export function SettingsSection({
  className,
  ...props
}: ComponentProps<'section'>) {
  return (
    <section
      className={cn(
        'mt-8 flex min-w-0 flex-col gap-4 scroll-mt-22 [&>[data-slot=separator]]:mb-1 [&>p]:max-w-[70ch] [&>p]:text-sm [&>p]:leading-[1.6] [&>p]:text-muted-foreground [&>form]:mt-0',
        className,
      )}
      {...props}
    />
  );
}

export function SettingsToolbar({
  className,
  ...props
}: ComponentProps<'div'>) {
  return (
    <div
      className={cn('mb-6 flex flex-wrap items-center gap-2', className)}
      {...props}
    />
  );
}

export function SettingsToolbarActions({
  className,
  ...props
}: ComponentProps<'div'>) {
  return (
    <div
      className={cn(
        'ml-auto flex w-full flex-wrap justify-end gap-2 md:w-auto',
        className,
      )}
      {...props}
    />
  );
}
