import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function PageTitle({
  className,
  children,
  ...props
}: ComponentProps<'h1'>) {
  return (
    <h1
      className={cn(
        'mb-3 text-[22px] font-semibold leading-[1.2] tracking-[-0.035em] text-balance',
        className,
      )}
      {...props}
    >
      {children}
    </h1>
  );
}

export function SectionTitle({
  className,
  children,
  ...props
}: ComponentProps<'h2'>) {
  return (
    <h2
      className={cn(
        'flex items-center gap-2 text-[15px] font-semibold',
        className,
      )}
      {...props}
    >
      {children}
    </h2>
  );
}

export function Subheading({
  className,
  children,
  ...props
}: ComponentProps<'h3'>) {
  return (
    <h3
      className={cn(
        'flex flex-wrap items-baseline gap-2 text-[15px] font-semibold',
        className,
      )}
      {...props}
    >
      {children}
    </h3>
  );
}

export function Intro({ className, ...props }: ComponentProps<'p'>) {
  return (
    <p
      className={cn(
        'max-w-[65ch] text-sm leading-[1.6] text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}
