export function isJson(str: string): boolean {
  const trimmed = str.trim();
  return (trimmed.startsWith('{') && trimmed.endsWith('}'))
      || (trimmed.startsWith('[') && trimmed.endsWith(']'));
}

export function formatJson(str: string): string {
  try {
    return JSON.stringify(JSON.parse(str), null, 2);
  } catch {
    return str;
  }
}

export function compressJson(str: string): string {
  try {
    return JSON.stringify(JSON.parse(str));
  } catch {
    return str;
  }
}
