import { useState, useEffect } from 'react';
import { diffLines, diffChars } from 'diff';
import { useDebounce } from './useDebounce';

export interface DiffChar {
  value: string;
  added?: boolean;
  removed?: boolean;
}

export interface DiffLine {
  content: string;
  type: 'equal' | 'insert' | 'delete';
  lineNumber: number;
  chars?: DiffChar[];
}

export interface DiffResult {
  originalLines: DiffLine[];
  modifiedLines: DiffLine[];
  stats: {
    additions: number;
    deletions: number;
    modifications: number;
  };
}

export interface DiffOptions {
  ignoreWhitespace: boolean;
  ignoreCase: boolean;
}

export function useDiff(
  originalText: string,
  modifiedText: string,
  options: DiffOptions
) {
  const [diffResult, setDiffResult] = useState<DiffResult>({
    originalLines: [],
    modifiedLines: [],
    stats: { additions: 0, deletions: 0, modifications: 0 }
  });

  // 问题4修复：智能防抖 - 小文本（<100行）无延迟，大文本100ms延迟
  const lineCount = Math.max(
    originalText.split('\n').length,
    modifiedText.split('\n').length
  );
  const debounceDelay = lineCount < 100 ? 0 : 100;

  const debouncedOriginal = useDebounce(originalText, debounceDelay);
  const debouncedModified = useDebounce(modifiedText, debounceDelay);

  // 预处理文本
  const preprocessText = (text: string): string => {
    let result = text;

    if (options.ignoreCase) {
      result = result.toLowerCase();
    }

    if (options.ignoreWhitespace) {
      result = result
        .split('\n')
        .map(line => line.trim().split(/\s+/).join(' '))
        .join('\n');
    }

    return result;
  };

  // 对单行进行字符级 diff
  const diffLineChars = (originalLine: string, modifiedLine: string): {
    originalChars: DiffChar[];
    modifiedChars: DiffChar[];
  } => {
    const changes = diffChars(originalLine, modifiedLine);

    const originalChars: DiffChar[] = [];
    const modifiedChars: DiffChar[] = [];

    changes.forEach(change => {
      if (change.removed) {
        originalChars.push({
          value: change.value,
          removed: true
        });
      } else if (change.added) {
        modifiedChars.push({
          value: change.value,
          added: true
        });
      } else {
        // 相同部分，两侧都添加
        originalChars.push({
          value: change.value
        });
        modifiedChars.push({
          value: change.value
        });
      }
    });

    return { originalChars, modifiedChars };
  };

  // 计算差异（使用防抖后的值）
  useEffect(() => {
    // 问题4修复：移除console.log以提升性能
    const preprocessedOriginal = preprocessText(debouncedOriginal);
    const preprocessedModified = preprocessText(debouncedModified);

    // 使用 diffLines 进行行级对比
    const changes = diffLines(preprocessedOriginal, preprocessedModified);

    const newOriginalLines: DiffLine[] = [];
    const newModifiedLines: DiffLine[] = [];
    let originalLineNum = 1;
    let modifiedLineNum = 1;
    let additions = 0;
    let deletions = 0;
    let modifications = 0;

    changes.forEach((part) => {
      // 问题4修复：正确处理空行
      // split('\n')会在末尾产生空字符串（如果原文本以\n结尾），
      // 但用户输入的空行应该保留
      let lines = part.value.split('\n');
      // 只移除因末尾\n产生的空字符串，保留中间的空行
      if (lines.length > 0 && lines[lines.length - 1] === '') {
        lines = lines.slice(0, -1);
      }

      if (part.removed) {
        // 删除的行：只在左侧显示
        lines.forEach((line) => {
          newOriginalLines.push({
            content: line,
            type: 'delete',
            lineNumber: originalLineNum++,
            chars: [{ value: line, removed: true }] // 整行标记为删除
          });
          // 右侧不添加占位符
        });
        deletions += lines.length;
      } else if (part.added) {
        // 新增的行：只在右侧显示
        lines.forEach((line) => {
          // 左侧不添加占位符
          newModifiedLines.push({
            content: line,
            type: 'insert',
            lineNumber: modifiedLineNum++,
            chars: [{ value: line, added: true }] // 整行标记为新增
          });
        });
        additions += lines.length;
      } else {
        // 相同的行：两侧都显示
        lines.forEach((line) => {
          newOriginalLines.push({
            content: line,
            type: 'equal',
            lineNumber: originalLineNum++
          });
          newModifiedLines.push({
            content: line,
            type: 'equal',
            lineNumber: modifiedLineNum++
          });
        });
      }
    });

    // 尝试对删除和新增的行进行配对（字符级对比）
    let i = 0;
    let j = 0;
    const alignedOriginal: DiffLine[] = [];
    const alignedModified: DiffLine[] = [];

    while (i < newOriginalLines.length || j < newModifiedLines.length) {
      const origLine = i < newOriginalLines.length ? newOriginalLines[i] : null;
      const modLine = j < newModifiedLines.length ? newModifiedLines[j] : null;

      if (!origLine) {
        // 右侧有行，左侧没有：添加空行到左侧
        alignedOriginal.push({
          content: '',
          type: 'equal',
          lineNumber: -1
        });
        alignedModified.push(modLine!);
        j++;
      } else if (!modLine) {
        // 左侧有行，右侧没有：添加空行到右侧
        alignedOriginal.push(origLine);
        alignedModified.push({
          content: '',
          type: 'equal',
          lineNumber: -1
        });
        i++;
      } else {
        // 两侧都有行
        if (origLine.type === 'delete' && modLine.type === 'insert') {
          // 这是一对修改的行，进行字符级对比
          const { originalChars, modifiedChars } = diffLineChars(origLine.content, modLine.content);

          alignedOriginal.push({
            ...origLine,
            chars: originalChars
          });
          alignedModified.push({
            ...modLine,
            chars: modifiedChars
          });
          modifications++;
          i++;
          j++;
        } else if (origLine.type === 'equal' && modLine.type === 'equal') {
          // 相同的行
          alignedOriginal.push(origLine);
          alignedModified.push(modLine);
          i++;
          j++;
        } else if (origLine.type === 'delete') {
          // 左侧删除，右侧对应位置是相同的行
          alignedOriginal.push(origLine);
          alignedModified.push({
            content: '',
            type: 'equal',
            lineNumber: -1
          });
          i++;
        } else {
          // 右侧新增，左侧对应位置是相同的行
          alignedOriginal.push({
            content: '',
            type: 'equal',
            lineNumber: -1
          });
          alignedModified.push(modLine);
          j++;
        }
      }
    }

    const result: DiffResult = {
      originalLines: alignedOriginal,
      modifiedLines: alignedModified,
      stats: {
        additions,
        deletions,
        modifications
      }
    };

    setDiffResult(result);

    // 问题4修复：移除console.log以提升性能
  }, [debouncedOriginal, debouncedModified, options.ignoreWhitespace, options.ignoreCase]);

  return diffResult;
}
