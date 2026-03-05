export interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
}

export function validateJson(jsonStr: string): ValidationError {
  try {
    JSON.parse(jsonStr);
    return { valid: true, error: null };
  } catch (e: any) {
    const errorStr = e.toString();
    const lineMatch = errorStr.match(/line (\d+)/);
    const columnMatch = errorStr.match(/column (\d+)/);

    return {
      valid: false,
      error: errorStr,
      line: lineMatch ? parseInt(lineMatch[1]) : undefined,
      column: columnMatch ? parseInt(columnMatch[1]) : undefined,
    };
  }
}

export function formatJson(jsonStr: string): string {
  const parsed = JSON.parse(jsonStr);
  return JSON.stringify(parsed, null, 2);
}

export function minifyJson(jsonStr: string): string {
  const parsed = JSON.parse(jsonStr);
  return JSON.stringify(parsed);
}

export function escapeJson(jsonStr: string): string {
  return jsonStr
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

export function unescapeJson(jsonStr: string): string {
  return jsonStr
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\r')
    .replace(/\\t/g, '\t')
    .replace(/\\"/g, '"')
    .replace(/\\\\/g, '\\');
}
