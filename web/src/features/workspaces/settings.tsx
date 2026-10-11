import { PageTitle } from '@/components/typography';
import { Link, useLocation, useRouteContext } from '@tanstack/react-router';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { LoadingState, Notice } from '@/components/feedback';
import { errorMessage, unwrap } from '@/lib/requests';
import { MembersSettings } from './members-settings';
import type { Schema } from './settings-shared';
import { CreateTeam, TeamsSettings } from './teams-settings';
import { WorkspaceGeneral } from './workspace-general';

export function WorkspaceSettings() {
  const { hash } = useLocation();
  const { workspaceId } = useRouteContext({ from: '/_app/$workspaceSlug' });
  const { account } = useAuth();
  const [detail, setDetail] = useState<Schema['WorkspaceDetail'] | null>(null);
  const [invitations, setInvitations] = useState<Schema['Invitation'][]>([]);
  const [error, setError] = useState('');
  const generation = useRef(0);
  const cancelPending = useCallback(() => {
    generation.current++;
  }, []);
  const load = useCallback(async () => {
    if (!workspaceId)
      throw new Error(
        'This workspace is unavailable or you no longer have access.',
      );
    const requestGeneration = ++generation.current;
    const result = unwrap(
      await api.GET('/api/v1/workspaces/{workspace_id}', {
        params: { path: { workspace_id: workspaceId } },
      }),
    );
    const invites =
      result.workspace.role === 'member'
        ? []
        : unwrap(
            await api.GET('/api/v1/workspaces/{workspace_id}/invitations', {
              params: { path: { workspace_id: workspaceId } },
            }),
          );
    if (requestGeneration !== generation.current) return;
    setDetail(result);
    setInvitations(invites);
  }, [workspaceId]);
  useEffect(() => {
    let active = true;
    setDetail(null);
    setInvitations([]);
    setError('');
    load().catch((error) => {
      if (active) setError(errorMessage(error));
    });
    return () => {
      active = false;
      cancelPending();
    };
  }, [load, cancelPending]);
  useEffect(() => {
    if (!hash || ['teams', 'create-team', 'people', 'members'].includes(hash))
      window.scrollTo(0, 0);
  }, [hash]);
  if (error)
    return (
      <>
        <PageTitle>Workspace unavailable</PageTitle>
        <Notice error>{error}</Notice>
        <Link to="/">Return to workspaces</Link>
      </>
    );
  if (!detail || detail.workspace.id !== workspaceId || !account)
    return <LoadingState>Loading workspace…</LoadingState>;
  const manage =
    detail.workspace.role === 'owner' || detail.workspace.role === 'admin';
  const props = { detail, manage, load };
  if (hash === 'teams') return <TeamsSettings {...props} />;
  if (hash === 'create-team' && manage) return <CreateTeam {...props} />;
  if (hash === 'people' || hash === 'members')
    return <MembersSettings {...props} invitations={invitations} />;
  return <WorkspaceGeneral {...props} />;
}
