import { SectionTitle } from '@/components/typography';
import {
  ActivityIcon,
  CircleAlertIcon,
  CircleCheckIcon,
  RefreshCwIcon,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Spinner } from '@/components/ui/spinner';
import { api } from './api/client';
import type { components } from './api/schema';

type Connection =
  | { state: 'loading' }
  | { state: 'ready'; health: components['schemas']['HealthResponse'] }
  | { state: 'error'; message: string };

export function SystemPage() {
  const [connection, setConnection] = useState<Connection>({
    state: 'loading',
  });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setConnection({ state: 'loading' });
    api
      .GET('/api/v1/health', { signal: controller.signal })
      .then(({ data, error }) => {
        if (controller.signal.aborted) return;
        setConnection(
          data
            ? { state: 'ready', health: data }
            : {
                state: 'error',
                message:
                  error?.message ??
                  'The server returned an unexpected response.',
              },
        );
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setConnection({
            state: 'error',
            message:
              'Could not reach the server. Check that the API is running, then try again.',
          });
      });
    return () => controller.abort();
    // oxlint-disable-next-line react/exhaustive-effect-dependencies -- The attempt counter explicitly triggers a fresh request on retry.
  }, [attempt]);

  return (
    <>
      <Card className="mt-8 max-w-sm w-full" aria-labelledby="connection-heading">
        <CardHeader>
          <CardTitle>
            <SectionTitle id="connection-heading">
              System connection
            </SectionTitle>
          </CardTitle>
          <CardDescription>
            Convex API and database availability
          </CardDescription>
          <CardAction>
            <Button
              variant="outline"
              type="button"
              disabled={connection.state === 'loading'}
              onClick={() => setAttempt(attempt + 1)}
            >
              {connection.state === 'loading' ? (
                <Spinner
                  data-icon="inline-start"
                  role="presentation"
                  aria-hidden="true"
                />
              ) : (
                <RefreshCwIcon data-icon="inline-start" />
              )}
              {connection.state === 'loading' ? 'Checking…' : 'Check again'}
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <Alert
            variant={connection.state === 'error' ? 'destructive' : 'default'}
            role="status"
            aria-live="polite"
          >
            {connection.state === 'ready' ? (
              <CircleCheckIcon aria-hidden="true" />
            ) : connection.state === 'error' ? (
              <CircleAlertIcon aria-hidden="true" />
            ) : (
              <ActivityIcon aria-hidden="true" />
            )}
            <AlertTitle>
              {connection.state === 'loading'
                ? 'Connecting to Convex'
                : connection.state === 'ready'
                  ? 'Healthy'
                  : 'Connection unavailable'}
            </AlertTitle>
            <AlertDescription>
              {connection.state === 'loading'
                ? 'Checking the APIconnection...'
                : connection.state === 'ready'
                  ? 'The API is responding and ready.'
                  : connection.message}
            </AlertDescription>
          </Alert>
        </CardContent>
        <CardFooter>
          <dl className="flex w-full flex-col gap-3 text-sm">
            <div className="flex justify-between gap-4">
              <dt>Application version</dt>
              <dd className="tabular-nums">
                {connection.state === 'ready' ? connection.health.version : '—'}
              </dd>
            </div>
            <Separator />
            <div className="flex justify-between gap-4">
              <dt>API contract</dt>
              <dd>
                <Button asChild variant="link" className="h-auto p-0">
                  <a href="/api/openapi.json">Open specification</a>
                </Button>
              </dd>
            </div>
          </dl>
        </CardFooter>
      </Card>
    </>
  );
}
