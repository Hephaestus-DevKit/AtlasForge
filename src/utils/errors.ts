export function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  if (error != null) {
    const rendered = String(error);
    if (rendered && rendered !== "[object Object]") {
      return rendered;
    }
  }
  return fallback;
}
