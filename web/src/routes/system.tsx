import { createFileRoute } from '@tanstack/react-router';
import { SystemPage } from '@/system-page';
export const Route = createFileRoute('/system')({
  component: () => (
    <div className="flex min-h-svh flex-col bg-sidebar p-5 md:px-8 md:py-6">
      <div className="flex flex-grow flex-col items-center justify-center p-4 text-sm text-muted-foreground">
        <SystemPage />
      </div>
    </div>
  ),
});
