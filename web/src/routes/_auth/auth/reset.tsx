import { createFileRoute } from '@tanstack/react-router';
import { Reset } from '@/features/auth/reset';
export const Route = createFileRoute('/_auth/auth/reset')({ component: Reset });
