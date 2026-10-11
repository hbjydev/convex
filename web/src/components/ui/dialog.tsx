import { Dialog as DialogPrimitive } from 'radix-ui';
import { XIcon } from 'lucide-react';
import type { ComponentProps } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogTitle = DialogPrimitive.Title;
export const DialogDescription = DialogPrimitive.Description;

export function DialogContent({
  children,
  className,
  ...props
}: ComponentProps<typeof DialogPrimitive.Content>) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/60" />
      <DialogPrimitive.Content
        data-slot="dialog-content"
        className={cn(
          'fixed top-1/2 left-1/2 z-50 -translate-x-1/2 -translate-y-1/2 max-h-[calc(100svh-48px)] w-[min(560px,calc(100%-32px))] overflow-y-auto rounded-xl bg-popover p-5 text-sm text-popover-foreground shadow-[0_12px_40px_rgb(0_0_0/18%)] [&_h2]:text-sm [&_h2]:font-medium [&_[role=alert]]:mt-3 [&_[role=status]]:mt-3',
          className,
        )}
        {...props}
      >
        {children}
        <DialogPrimitive.Close asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            className="absolute right-3 top-3"
            aria-label="Close dialog"
          >
            <XIcon />
          </Button>
        </DialogPrimitive.Close>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}
