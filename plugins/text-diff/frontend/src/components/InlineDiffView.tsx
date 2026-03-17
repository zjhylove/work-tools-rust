import { useMemo } from 'react';
import type { DiffLine } from '../hooks/useDiff';
import './InlineDiffView.css';

interface InlineDiffViewProps {
  originalLines: DiffLine[];
  modifiedLines: DiffLine[];
}

export function InlineDiffView({ originalLines, modifiedLines }: InlineDiffViewProps) {
  // 合并差异行用于行内显示
  const mergedLines = useMemo(() => {
    const result: Array<{
      original: DiffLine | null;
      modified: DiffLine | null;
    }> = [];

    const maxLength = Math.max(originalLines.length, modifiedLines.length);

    for (let i = 0; i < maxLength; i++) {
      const origLine = originalLines[i];
      const modLine = modifiedLines[i];

      // 如果两行都是相同的,只显示一次
      if (origLine?.type === 'equal' && modLine?.type === 'equal') {
        result.push({
          original: origLine,
          modified: null  // 不重复显示
        });
      } else {
        // 有差异,分别显示
        result.push({
          original: origLine,
          modified: modLine
        });
      }
    }

    return result;
  }, [originalLines, modifiedLines]);

  return (
    <div className="inline-diff-view">
      <div className="inline-header">
        <h3>对比视图</h3>
        <div className="inline-stats">
          <span className="stat-additions">+{originalLines.filter(l => l.type === 'insert').length}</span>
          <span className="stat-deletions">-{modifiedLines.filter(l => l.type === 'delete').length}</span>
        </div>
      </div>

      <div className="inline-content">
        {mergedLines.map((item, index) => {
          const origLine = item.original;
          const modLine = item.modified;

          // 相同的行,只显示一次
          if (origLine?.type === 'equal' && !modLine) {
            return (
              <div key={index} className="inline-line inline-equal" data-line-index={index}>
                <span className="line-marker"> </span>
                <span className="line-number">{origLine.lineNumber}</span>
                <span className="line-content">{origLine.content}</span>
              </div>
            );
          }

          // 删除的行
          if (origLine?.type === 'delete') {
            return (
              <div key={index} className="inline-line inline-delete" data-line-index={index}>
                <span className="line-marker">-</span>
                <span className="line-number">{origLine.lineNumber}</span>
                <span className="line-content">{origLine.content}</span>
              </div>
            );
          }

          // 新增的行
          if (modLine?.type === 'insert') {
            return (
              <div key={index} className="inline-line inline-insert" data-line-index={index}>
                <span className="line-marker">+</span>
                <span className="line-number">{modLine.lineNumber}</span>
                <span className="line-content">{modLine.content}</span>
              </div>
            );
          }

          // 其他情况
          return null;
        })}
      </div>
    </div>
  );
}
