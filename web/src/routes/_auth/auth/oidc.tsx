import { createFileRoute } from '@tanstack/react-router';
import { OidcOutcome } from '@/features/auth/oidc';
export const Route = createFileRoute('/_auth/auth/oidc')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { outcome: 'success' | 'failed' } => ({
    outcome: search.outcome === 'success' ? 'success' : 'failed',
  }),
  component: Outcome,
});
function Outcome() {
  return <OidcOutcome outcome={Route.useSearch().outcome} />;
}
