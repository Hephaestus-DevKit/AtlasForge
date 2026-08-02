export const APP_ROUTES = [
  "/",
  "/assets",
  "/repositories",
  "/tasks",
  "/knowledge",
  "/automations",
  "/settings",
] as const;

export type AppRoute = (typeof APP_ROUTES)[number];

export function currentRoute(): AppRoute {
  const value = window.location.hash.replace(/^#/, "") || "/";
  return APP_ROUTES.includes(value as AppRoute) ? value as AppRoute : "/";
}

export function navigateTo(route: AppRoute): void {
  if (window.location.hash === `#${route}`) return;
  window.location.hash = route;
}
