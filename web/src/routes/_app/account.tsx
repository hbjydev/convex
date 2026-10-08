import { createFileRoute } from '@tanstack/react-router';
import { AccountSettings } from '@/features/account/settings';

export const Route = createFileRoute('/_app/account')({
  component: AccountSettings,
});
