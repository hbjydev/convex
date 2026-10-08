import { Outlet } from "@tanstack/react-router";

export const AuthShell = () => (
  <div className="flex h-screen w-screen items-center justify-center bg-gray-50 dark:bg-gray-900">
    <Outlet />
  </div>
);
