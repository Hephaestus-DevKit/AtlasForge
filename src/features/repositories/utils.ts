export function repositoryName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
