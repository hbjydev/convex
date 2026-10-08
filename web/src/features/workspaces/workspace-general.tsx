import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Form, text } from '@/components/app-form';
import { Input } from '@/components/ui/input';
import { unwrap } from '@/lib/requests';
import { initials, type SettingsProps } from './settings-shared';

export function WorkspaceGeneral({ detail, manage, load }: SettingsProps) {
  const { reload } = useAuth();
  const workspace = detail.workspace;
  return (
    <div className="settings-narrow">
      <h1>Workspace</h1>
      <Form
        key={workspace.id}
        className="settings-profile-form"
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
        <div className="settings-panel">
          <div className="settings-row">
            <div>
              <span className="settings-label">Workspace icon</span>
              <p>Initials from your workspace name</p>
            </div>
            <span className="settings-workspace-icon" aria-hidden="true">
              {initials(workspace.name)}
            </span>
          </div>
          <div className="settings-row">
            <label className="settings-label" htmlFor="workspace-name">
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
          </div>
          <div className="settings-row">
            <label className="settings-label" htmlFor="workspace-url">
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
          </div>
        </div>
        {!manage && (
          <p className="muted">
            Only workspace owners and admins can change the name.
          </p>
        )}
      </Form>
      <section className="settings-block">
        <h2>Workspace access</h2>
        <div className="settings-panel">
          <div className="settings-row">
            <span className="settings-label">Your role</span>
            <span className="capitalize">{workspace.role}</span>
          </div>
          <div className="settings-row">
            <div>
              <span className="settings-label">Team visibility</span>
              <p>Members can access every team in this workspace.</p>
            </div>
            <span>Workspace</span>
          </div>
        </div>
      </section>
    </div>
  );
}
