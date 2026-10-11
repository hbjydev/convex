import { createFileRoute } from '@tanstack/react-router';
import { CreateWorkspace } from '@/features/workspaces/create';

export const Route = createFileRoute('/_app/workspaces/create')({
  component: CreateWorkspace,
});
