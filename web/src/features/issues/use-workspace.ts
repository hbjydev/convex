import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { errorMessage, unwrap } from '@/lib/requests';
import type { WorkspaceDetail } from './model';
export function useIssueWorkspace(slug: string) {
  const { account } = useAuth();
  const workspaceId = account?.workspaces.find(
    (space) => space.slug === slug,
  )?.id;
  const [detail, setDetail] = useState<WorkspaceDetail>();
  const [error, setError] = useState('');
  const generation = useRef(0);
  const load = useCallback(async () => {
    const current = ++generation.current;
    setDetail(undefined);
    setError('');
    if (!workspaceId) {
      setError('This workspace is unavailable or you no longer have access.');
      return;
    }
    try {
      const result = unwrap(
        await api.GET('/api/v1/workspaces/{workspace_id}', {
          params: { path: { workspace_id: workspaceId } },
        }),
      );
      if (current === generation.current) setDetail(result);
    } catch (reason) {
      if (current === generation.current) setError(errorMessage(reason));
    }
  }, [workspaceId]);
  const cancelPending = useCallback(() => {
    generation.current++;
  }, []);
  useEffect(() => {
    void load();
    return cancelPending;
  }, [load, cancelPending]);
  return {
    detail: detail?.workspace.id === workspaceId ? detail : undefined,
    error,
    retry: load,
  };
}
