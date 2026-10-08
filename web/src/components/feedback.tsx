import type { ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';
import { Spinner } from '@/components/ui/spinner';

export function Notice({
  children,
  error = false,
}: {
  children: ReactNode;
  error?: boolean;
}) {
  return (
    <Alert
      variant={error ? 'destructive' : 'default'}
      role={error ? 'alert' : 'status'}
    >
      <AlertDescription>{children}</AlertDescription>
    </Alert>
  );
}

export function LoadingState({ children }: { children: ReactNode }) {
  return (
    <div className="flex max-w-xl flex-col gap-6" aria-busy="true">
      <p
        role="status"
        className="flex items-center gap-2 text-sm text-muted-foreground"
      >
        <Spinner role="presentation" aria-hidden="true" />
        {children}
      </p>
      <div className="flex flex-col gap-3" aria-hidden="true">
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-4/5" />
      </div>
    </div>
  );
}
