import { useLocation } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { useAuth } from '@/auth';

const STORAGE_KEY = 'convex.current-workspace';

export function useCurrentWorkspace() {
  const { account } = useAuth();
  const { pathname } = useLocation();
  const [lastWorkspaceId, setLastWorkspaceId] = useState(() => {
    try {
      return localStorage.getItem(STORAGE_KEY);
    } catch {
      return null;
    }
  });
  const routeWorkspace = account?.workspaces.find(
    (workspace) =>
      pathname === `/${workspace.slug}` ||
      pathname.startsWith(`/${workspace.slug}/`),
  );
  const routeWorkspaceId = routeWorkspace?.id;
  useEffect(() => {
    if (!routeWorkspaceId) return;
    setLastWorkspaceId(routeWorkspaceId);
    try {
      localStorage.setItem(STORAGE_KEY, routeWorkspaceId);
    } catch {
      // Navigation remains available when browser storage is disabled.
    }
  }, [routeWorkspaceId]);
  return (
    routeWorkspace ??
    account?.workspaces.find((workspace) => workspace.id === lastWorkspaceId) ??
    account?.workspaces[0]
  );
}
