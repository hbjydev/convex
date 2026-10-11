import { cn } from 'cn';
import type * as React from 'react';

function Textarea({
  className,
  variant = 'default',
  ...props
}: React.ComponentProps<'textarea'> & { variant?: 'default' | 'inline' }) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        variant === 'inline'
          ? 'block field-sizing-content min-h-0 w-full resize-none overflow-hidden rounded-none border-0 bg-transparent p-0 text-[length:inherit] font-[inherit] leading-[inherit] tracking-[inherit] shadow-none outline-none focus-visible:rounded-lg focus-visible:outline-1 focus-visible:outline-ring focus-visible:outline-offset-8'
          : 'flex field-sizing-content min-h-16 w-full rounded-lg border border-input bg-transparent px-2.5 py-2 text-base transition-colors outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40',
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
