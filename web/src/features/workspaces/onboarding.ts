import type { components } from '@/api/schema';

export function requiresWorkspaceCreation(
  account: components['schemas']['AccountResponse'],
) {
  return (
    account.user.is_instance_admin &&
    !account.workspaces.some((workspace) => workspace.role === 'owner')
  );
}
