import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function DirectoryList({ className, ...props }: ComponentProps<'ul'>) {
  return (
    <ul
      className={cn(
        'm-0 list-none p-0 [&>li]:flex [&>li]:items-center [&>li]:justify-between [&>li]:gap-5 [&>li]:border-b [&>li]:py-4 [&>li:first-child]:pt-0 max-[600px]:[&>li]:flex-col max-[600px]:[&>li]:items-start max-[600px]:[&>li]:gap-3 [&>li>div:first-child]:flex [&>li>div:first-child]:min-w-0 [&>li>div:first-child]:flex-col [&>li>div:first-child]:gap-1 [&_strong]:text-sm [&_strong]:font-semibold [&_span:not([data-slot])]:text-[13px] [&_span:not([data-slot])]:leading-[1.6] [&_span:not([data-slot])]:text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}
