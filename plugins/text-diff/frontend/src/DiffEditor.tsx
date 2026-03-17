import { useState, useEffect, useRef, useCallback } from 'react';
import { diffLines } from 'diff';
import './DiffEditor.css';

interface DiffEditorProps {
  originalText: string;
  modifiedText: string;
  options: {
    ignoreWhitespace: boolean;
    ignoreCase: boolean;
  };
  onEditorReady?: (editor: { goToDiff: (direction: 'next' | 'previous') => void }) => void;
}

interface DiffLine {
  content: string;
  type: 'delete' | 'insert' | 'equal' | 'empty';
  lineNumber?: number;
}

export function DiffEditor({
  originalText,
  modifiedText,
  options,
  onEditorReady
}: DiffEditorProps) {
  const [originalLines, setOriginalLines] = useState<DiffLine[]>([]);
  const [modifiedLines, setModifiedLines] = useState<DiffLine[]>([]);
  const diffIndicesRef = useRef<number[]>([]);

  // 预处理文本
  const preprocessText = useCallback((text: string): string => {
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
  }, [options.ignoreCase, options.ignoreWhitespace]);

  // 计算差异
  useEffect(() => {
    console.log('[DiffEditor] Computing diff...');

    const preprocessedOriginal = preprocessText(originalText);
    const preprocessedModified = preprocessText(modifiedText);

    const changes = diffLines(preprocessedOriginal, preprocessedModified);

    const newOriginalLines: DiffLine[] = [];
    const newModifiedLines: DiffLine[] = [];
    const newDiffIndices: number[] = [];
    let diffIndex = 0;
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
            type: 'empty'
          });
          newDiffIndices.push(diffIndex);
        });
        diffIndex++;
      } else if (part.added) {
        // 新增的行
        lines.forEach((line) => {
          newOriginalLines.push({
            content: '',
            type: 'empty'
          });
          newModifiedLines.push({
            content: line,
            type: 'insert',
            lineNumber: modifiedLineNum++
          });
          newDiffIndices.push(diffIndex);
        });
        diffIndex++;
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

    setOriginalLines(newOriginalLines);
    setModifiedLines(newModifiedLines);
    diffIndicesRef.current = newDiffIndices;

    console.log('[DiffEditor] Diff computed:', {
      totalChanges: diffIndex,
      originalLines: newOriginalLines.length,
      modifiedLines: newModifiedLines.length
    });
  }, [originalText, modifiedText, preprocessText]);

  // 暴露导航方法
  useEffect(() => {
    if (onEditorReady) {
      onEditorReady({
        goToDiff: (direction: 'next' | 'previous') => {
          const indices = diffIndicesRef.current;
          if (indices.length === 0) return;

          console.log('[DiffEditor] goToDiff:', direction, 'Total diffs:', indices.length);
          // TODO: 实现差异导航高亮
        }
      });
    }
  }, [onEditorReady]);

  // 渲染行
  const renderLine = (line: DiffLine) => {
    return (
      <div
        className={`diff-line diff-line-${line.type}`}
        data-line-number={line.lineNumber || ''}
      >
        <span className="line-number">{line.lineNumber || ''}</span>
        <span className="line-content">{line.content || '\u00A0'}</span>
      </div>
    );
  };

  return (
    <div className="diff-editor-container">
      <div className="diff-editor-pane">
        <div className="pane-header">原始文件</div>
        <div className="pane-content original-pane">
          {originalLines.map((line, index) => (
            <div key={index}>{renderLine(line)}</div>
          ))}
        </div>
      </div>

      <div className="diff-divider" />

      <div className="diff-editor-pane">
        <div className="pane-header">修改后的文件</div>
        <div className="pane-content modified-pane">
          {modifiedLines.map((line, index) => (
            <div key={index}>{renderLine(line)}</div>
          ))}
        </div>
      </div>
    </div>
  );
}
