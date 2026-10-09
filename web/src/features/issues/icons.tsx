import {
  CircleDashedIcon,
  CircleIcon,
  CircleDotIcon,
  CircleCheckIcon,
  CircleXIcon,
  EllipsisIcon,
  SignalLowIcon,
  SignalMediumIcon,
  SignalHighIcon,
  AlertCircleIcon,
} from 'lucide-react';
import type { Issue } from './model';
export function StatusIcon({ category }: { category?: string }) {
  const Icon =
    category === 'backlog'
      ? CircleDashedIcon
      : category === 'started'
        ? CircleDotIcon
        : category === 'completed'
          ? CircleCheckIcon
          : category === 'canceled'
            ? CircleXIcon
            : CircleIcon;
  return <Icon aria-hidden="true" data-status-category={category} />;
}
export function PriorityIcon({ priority }: { priority: Issue['priority'] }) {
  const Icon =
    priority === 'urgent'
      ? AlertCircleIcon
      : priority === 'high'
        ? SignalHighIcon
        : priority === 'medium'
          ? SignalMediumIcon
          : priority === 'low'
            ? SignalLowIcon
            : EllipsisIcon;
  return <Icon aria-hidden="true" />;
}
