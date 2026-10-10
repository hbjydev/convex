import type { ReactNode } from 'react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from '@/components/ui/dropdown-menu';
export function PropertyMenu({
  label,
  value,
  options,
  onChange,
  icon,
  disabled = false,
  inline = false,
  onCloseAutoFocus,
}: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  icon: ReactNode;
  disabled?: boolean;
  inline?: boolean;
  onCloseAutoFocus?: (event: Event) => void;
}) {
  const selected = options.find((option) => option.value === value);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant={inline ? 'ghost' : 'secondary'}
          size="sm"
          className={cn('issue-property', inline && 'issue-property-inline')}
          disabled={disabled}
          aria-label={`${label}: ${selected?.label ?? 'Choose'}`}
        >
          {icon}
          {selected?.label ?? label}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        className="min-w-48"
        onCloseAutoFocus={onCloseAutoFocus}
      >
        <DropdownMenuRadioGroup value={value} onValueChange={onChange}>
          {options.map((option) => (
            <DropdownMenuRadioItem key={option.value} value={option.value}>
              {option.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
