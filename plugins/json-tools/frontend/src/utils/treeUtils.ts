export type JsonPath = Array<string | number>;

export function getValueByPath(obj: any, path: JsonPath): any {
  return path.reduce((current, key) => current?.[key], obj);
}

export function deleteByPath(obj: any, path: JsonPath): any {
  if (path.length === 0) return obj;

  const [key, ...rest] = path;

  if (rest.length === 0) {
    if (Array.isArray(obj)) {
      return obj.filter((_, i) => i !== key);
    } else {
      const { [key]: _, ...result } = obj;
      return result;
    }
  }

  if (Array.isArray(obj)) {
    return obj.map((item, i) =>
      i === key ? deleteByPath(item, rest) : item
    );
  } else {
    return {
      ...obj,
      [key]: obj[key] !== undefined ? deleteByPath(obj[key], rest) : undefined
    };
  }
}

export function expandAll(data: any): Record<string, boolean> {
  const result: Record<string, boolean> = { 'root': true };

  function traverse(obj: any, path: (string | number)[]) {
    const pathStr = path.join('.');
    result[pathStr] = true;

    if (Array.isArray(obj)) {
      obj.forEach((item, i) => traverse(item, [...path, i]));
    } else if (typeof obj === 'object' && obj !== null) {
      Object.keys(obj).forEach(key => {
        traverse(obj[key], [...path, key]);
      });
    }
  }

  traverse(data, []);
  return result;
}
