import { createFileRoute } from '@tanstack/react-router';
import { AccountSettings } from '@/features/account/settings';

export const Route = createFileRoute('/_app/account')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { method?: 'linked' | 'proof' } => ({
    method:
      search.method === 'linked' || search.method === 'proof'
        ? search.method
        : undefined,
  }),
  component: AccountSettings,
});
