import { useRouter } from '@tanstack/react-router';
import { useSyncExternalStore } from 'react';
import { api } from '@/api/client';
import type { components } from '@/api/schema';
import { unwrap } from '@/lib/requests';

type Schema = components['schemas'];
export type AuthSnapshot = {
  account: Schema['AccountResponse'] | null;
  bootstrap: Schema['BootstrapInfo'];
};

export function createAuthStore() {
  let snapshot: AuthSnapshot | null = null;
  let loading: Promise<AuthSnapshot> | undefined;
  const listeners = new Set<() => void>();
  const reload = async () => {
    const [setup, current] = await Promise.all([
      api.GET('/api/v1/auth/bootstrap'),
      api.GET('/api/v1/auth/me'),
    ]);
    const bootstrap = unwrap(setup);
    const account = current.response.status === 401 ? null : unwrap(current);
    snapshot = { bootstrap, account };
    for (const listener of listeners) listener();
    return snapshot;
  };
  return {
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    ensureLoaded() {
      if (snapshot) return Promise.resolve(snapshot);
      loading ??= reload().finally(() => {
        loading = undefined;
      });
      return loading;
    },
    reload,
  };
}

export type AuthStore = ReturnType<typeof createAuthStore>;

export function useAuth() {
  const router = useRouter();
  const auth = router.options.context.auth;
  const snapshot = useSyncExternalStore(auth.subscribe, auth.getSnapshot);
  if (!snapshot) throw new Error('Authentication has not loaded');
  return {
    ...snapshot,
    reload: async () => {
      const updated = await auth.reload();
      await router.invalidate();
      return updated.account;
    },
  };
}
