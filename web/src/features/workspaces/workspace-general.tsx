import {
  SettingsPage,
  SettingsPanel,
  SettingsRow,
  SettingsBlock,
} from '@/components/settings-layout';
import { PageTitle, SectionTitle } from '@/components/typography';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Form, text } from '@/components/app-form';
import { Input } from '@/components/ui/input';
import { unwrap } from '@/lib/requests';
import { initials, type SettingsProps } from './settings-shared';
import { DeleteWorkspace } from './delete-workspace';

export function WorkspaceGeneral({ detail, manage, load }: SettingsProps) {
  const { reload } = useAuth();
  const workspace = detail.workspace;
  return (
    <SettingsPage>
      <PageTitle>Workspace</PageTitle>
      <Form
        key={workspace.id}
        className="mt-0 max-w-none [&>fieldset>[data-slot=field-group]]:gap-4 [&>fieldset>[data-slot=field-group]>button]:self-end"
        hideSubmit={!manage}
        submit="Save changes"
        action={async (form) => {
          const result = unwrap(
            await api.POST('/api/v1/workspaces/{workspace_id}/rename', {
              params: { path: { workspace_id: workspace.id } },
              body: { name: text(form, 'name') },
            }),
          );
          await load();
          await reload();
          return result.message;
        }}
      >
        <SettingsPanel>
          <SettingsRow>
            <div>
              <span className="mb-0.5 block text-[13px] font-medium">
                Workspace icon
              </span>
              <p>Initials from your workspace name</p>
            </div>
            <span
              className="inline-flex size-8 items-center justify-center rounded-lg bg-primary text-[11px] font-semibold text-primary-foreground"
              aria-hidden="true"
            >
              {initials(workspace.name)}
            </span>
          </SettingsRow>
          <SettingsRow>
            <label
              className="mb-0.5 block text-[13px] font-medium"
              htmlFor="workspace-name"
            >
              Name
            </label>
            <Input
              className="md:w-64"
              id="workspace-name"
              name="name"
              aria-label="Workspace name"
              defaultValue={workspace.name}
              required
              maxLength={100}
              disabled={!manage}
            />
          </SettingsRow>
          <SettingsRow>
            <label
              className="mb-0.5 block text-[13px] font-medium"
              htmlFor="workspace-url"
            >
              URL
            </label>
            <Input
              className="md:w-64"
              id="workspace-url"
              aria-label="Workspace URL"
              value={`${window.location.host}/${workspace.slug}`}
              readOnly
              title="Workspace URLs stay fixed when the name changes."
            />
          </SettingsRow>
        </SettingsPanel>
        {!manage && (
          <p className="text-[13px] leading-[1.6] text-muted-foreground">
            Only workspace owners and admins can change the name.
          </p>
        )}
      </Form>
      <SettingsBlock>
        <SectionTitle>Workspace access</SectionTitle>
        <SettingsPanel>
          <SettingsRow>
            <span className="mb-0.5 block text-[13px] font-medium">
              Your role
            </span>
            <span className="capitalize">{workspace.role}</span>
          </SettingsRow>
          <SettingsRow>
            <div>
              <span className="mb-0.5 block text-[13px] font-medium">
                Team visibility
              </span>
              <p>Members can access every team in this workspace.</p>
            </div>
            <span>Workspace</span>
          </SettingsRow>
        </SettingsPanel>
      </SettingsBlock>
      {workspace.role === 'owner' && (
        <DeleteWorkspace key={workspace.id} workspace={workspace} />
      )}
    </SettingsPage>
  );
}
