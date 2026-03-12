export interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
  suggestion?: string;
}

/**
 * 将字符位置转换为行号和列号
 */
function getPositionInfo(jsonStr: string, position: number): { line: number; column: number } {
  const lines = jsonStr.substring(0, position).split('\n');
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1
  };
}

/**
 * 通过逐步解析 JSON 来找到错误位置
 * 这个方法不依赖于浏览器的错误消息格式
 */
function findErrorPosition(jsonStr: string): { line?: number; column?: number; suggestion?: string } | null {
  const lines = jsonStr.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // 检查缺少逗号的情况 1: 对象属性值后缺少逗号
    // "key": "value"
    //     "nextKey": ...
    if (trimmed.endsWith('"') || trimmed.match(/^\d+$/) || trimmed === 'true' || trimmed === 'false' || trimmed === 'null') {
      // 检查下一行是否是新的属性或数组元素
      if (i < lines.length - 1) {
        const nextLine = lines[i + 1].trim();
        // 下一行以引号开头(新属性)或 { [(新对象/数组)
        if ((nextLine.startsWith('"') && !trimmed.endsWith(',')) ||
            (nextLine.startsWith('{') && !trimmed.endsWith(',')) ||
            (nextLine.startsWith('[') && !trimmed.endsWith(','))) {
          // 确认当前行不在对象/数组末尾(后面没有 } 或 ])
          const remainingText = jsonStr.split('\n').slice(i + 1).join('\n');
          const nextClosing = remainingText.search(/[}\]]/);

          if (nextClosing > 0) {
            // 找到当前行最后一个有意义的字符位置
            let endPos = line.length;
            if (trimmed.endsWith('"')) {
              // 如果以引号结尾,找到最后一个引号的位置
              endPos = line.lastIndexOf('"') + 1;
            }

            return {
              line: i + 1, // 指向当前行(需要加逗号的地方)
              column: endPos + 1, // 当前行末尾位置
              suggestion: '在该行末尾添加逗号'
            };
          }
        }
      }
    }

    // 检查对象或数组中缺少逗号: } { 或 ] [
    if ((line.includes('}') && line.includes('{')) ||
        (line.includes(']') && line.includes('['))) {
      const match = line.match(/[}\]]\s*[{[]/);
      if (match && !line.includes(',')) {
        const pos = line.indexOf(match[0]);
        return {
          line: i + 1,
          column: pos + 1,
          suggestion: '在两个对象/数组之间添加逗号'
        };
      }
    }
  }

  return null;
}

/**
 * 解析 JSON 错误消息,提取位置信息
 */
function parseErrorPosition(errorMessage: string, jsonStr: string): { line?: number; column?: number } {
  console.log('Parsing error:', errorMessage); // 调试日志

  // Firefox: "JSON.parse: expected property name or '}' at line 2 column 3 of the JSON data"
  const firefoxMatch = errorMessage.match(/line (\d+) column (\d+)/);
  if (firefoxMatch) {
    return {
      line: parseInt(firefoxMatch[1]),
      column: parseInt(firefoxMatch[2])
    };
  }

  // Chrome/Safari: "Unexpected token } in JSON at position 15"
  // 或 "Unexpected end of JSON input"
  const chromeMatch = errorMessage.match(/position (\d+)/);
  if (chromeMatch) {
    const pos = parseInt(chromeMatch[1]);
    return getPositionInfo(jsonStr, pos);
  }

  // Edge/其他: "SyntaxError: Expected ',' or '}' at position 25"
  // 或者没有明确位置信息的错误
  const edgeMatch = errorMessage.match(/position (\d+)/);
  if (edgeMatch) {
    const pos = parseInt(edgeMatch[1]);
    return getPositionInfo(jsonStr, pos);
  }

  // 尝试从 "at line X" 格式提取
  const lineMatch = errorMessage.match(/at line (\d+)/);
  if (lineMatch) {
    return { line: parseInt(lineMatch[1]) };
  }

  // 如果浏览器错误消息中没有位置信息,尝试手动分析
  console.log('No position in browser error, trying manual analysis'); // 调试日志
  const manualPosition = findErrorPosition(jsonStr);
  if (manualPosition) {
    const result: { line?: number; column?: number; suggestion?: string } = {
      line: manualPosition.line,
      column: manualPosition.column
    };
    if (manualPosition.suggestion) {
      result.suggestion = manualPosition.suggestion;
    }
    return result;
  }

  console.log('No position found'); // 调试日志
  return {};
}

/**
 * 提取更有用的错误描述
 */
function getErrorDescription(errorMessage: string): string {
  // 去掉前缀 (如 "SyntaxError: ", "JSON.parse: ")
  const cleanError = errorMessage
    .replace(/^SyntaxError:\s*/, '')
    .replace(/^JSON\.parse:\s*/, '')
    .replace(/^JSON Parse error:\s*/, '') // 处理 Tauri 环境的错误格式
    .replace(/\s+at\s+(line\s+\d+)?(column\s+\d+)?(of\s+the\s+JSON\s+data)?$/gi, '');

  // 翻译常见错误为中文
  const translations: Record<string, string> = {
    'Unexpected end of JSON input': 'JSON 结束 unexpectedly(可能缺少闭合括号或引号)',
    'Unexpected token': '意外的 token(语法错误)',
    "Expected property name or '}'": '期望属性名或 "}"',
    "Expected ',' or '}' after property value": '属性值后期望 "," 或 "}"',
    "Expected ',' or ']'": '期望 "," 或 "]"',
    'Expected double-quoted property name': '期望双引号括起来的属性名',
    'Unexpected number': '意外的数字',
    'Unexpected string': '意外的字符串',
    'Unexpected identifier': '意外的标识符',
    "Expected '}'": '期望 "}" (可能缺少逗号或闭合括号)',
    "Expected ',' or '}'": '期望 "," 或 "}" (可能缺少逗号)',
  };

  for (const [en, zh] of Object.entries(translations)) {
    if (cleanError.includes(en)) {
      return cleanError.replace(en, zh);
    }
  }

  return cleanError;
}

export function validateJson(jsonStr: string): ValidationError {
  try {
    JSON.parse(jsonStr);
    return { valid: true, error: null };
  } catch (e: any) {
    const errorStr = e.toString();
    console.log('Raw error:', e); // 调试
    console.log('Error string:', errorStr); // 调试
    console.log('Error message:', e.message); // 调试

    const positionInfo = parseErrorPosition(errorStr, jsonStr);
    const errorDescription = getErrorDescription(errorStr);

    console.log('Parsed position info:', positionInfo); // 调试

    return {
      valid: false,
      error: errorDescription,
      ...positionInfo
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
