import { SearchIcon } from 'lucide-react';
import type { components } from '@/api/schema';
import { Input } from '@/components/ui/input';

export type Schema = components['schemas'];
export type SettingsProps = {
  detail: Schema['WorkspaceDetail'];
  manage: boolean;
  load: () => Promise<void>;
};

export function SettingsSearch({
  value,
  onChange,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
}) {
  return (
    <div className="settings-search">
      <SearchIcon aria-hidden="true" />
      <Input
        className="pl-8"
        type="search"
        aria-label={label}
        placeholder={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

export function initials(name: string) {
  return name
    .trim()
    .split(/\s+/)
    .map((part) => part[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();
}
