import { useEffect, useRef, useState, type SubmitEvent } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { toast } from 'sonner';
import { api } from '@/api/client';
import type { components } from '@/api/schema';
import { useAuth } from '@/auth';
import { Notice } from '@/components/feedback';
import {
  SettingsBlock,
  SettingsPanel,
  SettingsRow,
} from '@/components/settings-layout';
import { SectionTitle } from '@/components/typography';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSet,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';
import { errorMessage, unwrap } from '@/lib/requests';

export function DeleteWorkspace({
  workspace,
}: {
  workspace: components['schemas']['Workspace'];
}) {
  const { bootstrap, reload } = useAuth();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  const [pending, setPending] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState('');
  const [deleted, setDeleted] = useState(false);
  const codeInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (email) codeInput.current?.focus();
  }, [email]);

  async function sendCode() {
    setSending(true);
    setError('');
    try {
      const result = unwrap(
        await api.POST('/api/v1/workspaces/{workspace_id}/deletion-code', {
          params: { path: { workspace_id: workspace.id } },
        }),
      );
      setEmail(result.email);
      setCode('');
      toast.success('Deletion verification code sent');
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setSending(false);
    }
  }

  async function finishDeletion() {
    await reload();
    await navigate({ to: '/', replace: true });
    toast.success('Workspace deleted');
  }

  async function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (pending || sending || !acknowledged || !email) return;
    setPending(true);
    setError('');
    try {
      unwrap(
        await api.POST('/api/v1/workspaces/{workspace_id}/delete', {
          params: { path: { workspace_id: workspace.id } },
          body: { code, acknowledged },
        }),
      );
      setDeleted(true);
      await finishDeletion();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <SettingsBlock>
      <SectionTitle>Danger zone</SectionTitle>
      <SettingsPanel>
        <SettingsRow>
          <div>
            <span className="mb-0.5 block text-[13px] font-medium">
              Delete workspace
            </span>
            <p>Permanently delete this workspace and all its data.</p>
            {!bootstrap.mail_available && (
              <p className="mt-2">
                Email delivery must be configured by your instance administrator
                before you can delete a workspace.
              </p>
            )}
          </div>
          <Dialog
            open={open}
            onOpenChange={(value) => {
              if (pending || sending || deleted) return;
              setOpen(value);
              if (value) {
                setEmail('');
                setCode('');
                setAcknowledged(false);
                setError('');
                void sendCode();
              }
            }}
          >
            <DialogTrigger asChild>
              <Button
                variant="destructive"
                disabled={!bootstrap.mail_available}
              >
                Delete workspace
              </Button>
            </DialogTrigger>
            <DialogContent
              className="max-w-[480px]"
              onInteractOutside={(event) => {
                if (pending || sending || deleted) event.preventDefault();
              }}
              onEscapeKeyDown={(event) => {
                if (pending || sending || deleted) event.preventDefault();
              }}
            >
              <DialogTitle>Verify workspace deletion</DialogTitle>
              <DialogDescription className="mt-6 text-[13px] leading-relaxed text-muted-foreground">
                Deleting{' '}
                <strong className="text-foreground">{workspace.name}</strong>{' '}
                permanently removes all its teams, issues, activity,
                memberships, and invitations. This cannot be undone. Your
                account and other workspaces remain available.
              </DialogDescription>
              {deleted ? (
                <div className="mt-6 flex flex-col gap-4">
                  <p>The workspace has been deleted.</p>
                  <Button
                    disabled={pending}
                    onClick={() => {
                      setPending(true);
                      setError('');
                      finishDeletion()
                        .catch((error) => setError(errorMessage(error)))
                        .finally(() => setPending(false));
                    }}
                  >
                    Back to app
                  </Button>
                  {error && <Notice error>{error}</Notice>}
                </div>
              ) : (
                <form
                  className="mt-6"
                  onSubmit={submit}
                  aria-busy={pending || sending}
                >
                  <FieldSet disabled={pending || sending}>
                    <FieldGroup className="gap-5">
                      {email ? (
                        <>
                          <Field>
                            <FieldLabel htmlFor="workspace-deletion-code">
                              Enter deletion code sent to {email}
                            </FieldLabel>
                            <Input
                              id="workspace-deletion-code"
                              ref={codeInput}
                              value={code}
                              onChange={(event) =>
                                setCode(event.target.value.toUpperCase())
                              }
                              required
                              minLength={8}
                              maxLength={8}
                              pattern="[A-Fa-f0-9]{8}"
                              autoComplete="one-time-code"
                              autoCapitalize="characters"
                              spellCheck={false}
                              placeholder="Enter the deletion code"
                              aria-describedby="workspace-deletion-code-hint"
                            />
                            <FieldDescription id="workspace-deletion-code-hint">
                              Codes expire after 10 minutes. Sending a new code
                              replaces the previous one.
                            </FieldDescription>
                          </Field>
                          <Field orientation="horizontal">
                            <input
                              id="workspace-deletion-acknowledgment"
                              type="checkbox"
                              required
                              checked={acknowledged}
                              onChange={(event) =>
                                setAcknowledged(event.target.checked)
                              }
                              className="mt-0.5 size-4 shrink-0 accent-primary focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-2"
                            />
                            <FieldLabel htmlFor="workspace-deletion-acknowledgment">
                              I acknowledge that all workspace data will be
                              deleted and want to proceed.
                            </FieldLabel>
                          </Field>
                          <Button
                            type="submit"
                            variant="destructive"
                            className="w-full min-h-11"
                            disabled={
                              !acknowledged ||
                              code.length !== 8 ||
                              pending ||
                              sending
                            }
                          >
                            {pending && (
                              <Spinner
                                data-icon="inline-start"
                                aria-hidden="true"
                              />
                            )}
                            {pending
                              ? 'Deleting workspace…'
                              : 'Delete my workspace'}
                          </Button>
                        </>
                      ) : (
                        <FieldDescription>
                          Send a verification code to your account’s email
                          address to continue.
                        </FieldDescription>
                      )}
                      <Button
                        type="button"
                        variant={email ? 'ghost' : 'outline'}
                        disabled={pending || sending}
                        onClick={() => void sendCode()}
                      >
                        {sending && (
                          <Spinner
                            data-icon="inline-start"
                            aria-hidden="true"
                          />
                        )}
                        {sending
                          ? 'Sending code…'
                          : email
                            ? 'Send a new code'
                            : 'Send verification code'}
                      </Button>
                    </FieldGroup>
                  </FieldSet>
                  {error && <Notice error>{error}</Notice>}
                </form>
              )}
            </DialogContent>
          </Dialog>
        </SettingsRow>
      </SettingsPanel>
    </SettingsBlock>
  );
}
