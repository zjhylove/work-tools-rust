import { useState, useEffect } from 'react';
import { diffLines } from 'diff';

export interface DiffLine {
  content: string;
  type: 'equal' | 'insert' | 'delete';
  lineNumber: number;
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

  // 计算差异
  useEffect(() => {
    console.log('[useDiff] Calculating diff...');

    const preprocessedOriginal = preprocessText(originalText);
    const preprocessedModified = preprocessText(modifiedText);

    // 使用 diff 库计算差异
    const changes = diffLines(preprocessedOriginal, preprocessedModified);

    const newOriginalLines: DiffLine[] = [];
    const newModifiedLines: DiffLine[] = [];
    let originalLineNum = 1;
    let modifiedLineNum = 1;

    changes.forEach((part) => {
      const lines = part.value.split('\n').filter(line => line !== '');

      if (part.removed) {
        // 删除的行
        lines.forEach((line) => {
          newOriginalLines.push({
            content: line,
            type: 'delete',
            lineNumber: originalLineNum++
          });
          newModifiedLines.push({
            content: '',
            type: 'equal',
            lineNumber: -1  // 占位符
          });
        });
      } else if (part.added) {
        // 新增的行
        lines.forEach((line) => {
          newOriginalLines.push({
            content: '',
            type: 'equal',
            lineNumber: -1  // 占位符
          });
          newModifiedLines.push({
            content: line,
            type: 'insert',
            lineNumber: modifiedLineNum++
          });
        });
      } else {
        // 相同的行
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

    // 计算统计
    const additions = newModifiedLines.filter(l => l.type === 'insert').length;
    const deletions = newOriginalLines.filter(l => l.type === 'delete').length;
    const modifications = Math.min(additions, deletions);

    const result: DiffResult = {
      originalLines: newOriginalLines,
      modifiedLines: newModifiedLines,
      stats: {
        additions: additions - modifications,
        deletions: deletions - modifications,
        modifications
      }
    };

    setDiffResult(result);

    console.log('[useDiff] Diff calculated:', {
      changes: changes.length,
      additions: result.stats.additions,
      deletions: result.stats.deletions,
      modifications: result.stats.modifications
    });
  }, [originalText, modifiedText, options.ignoreWhitespace, options.ignoreCase]);

  return diffResult;
}
