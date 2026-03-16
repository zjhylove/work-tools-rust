import { useEffect, useRef, useState } from 'react';
import * as monaco from 'monaco-editor';
import { diffLines } from 'diff';
import './DiffEditor.css';

// 配置 Monaco Worker
function configureMonacoWorkers() {
  if ((window as any).MonacoEnvironment) return;

  class DummyWorker {
    onmessage: ((message: any) => void) | null = null;
    addEventListener(_type: string, _listener: any) {}
    removeEventListener(_type: string, _listener: any) {}
    postMessage(_message: any) {}
    terminate() {}
  }

  (window as any).MonacoEnvironment = {
    getWorker: function (_moduleId: string, _label: string) {
      return new DummyWorker() as any;
    }
  };
}

interface DiffEditorProps {
  originalText: string;
  modifiedText: string;
  options: {
    ignoreWhitespace: boolean;
    ignoreCase: boolean;
  };
  onEditorReady?: (editor: any) => void;
}

export function DiffEditor({
  originalText,
  modifiedText,
  options: _options,
  onEditorReady: _onEditorReady
}: DiffEditorProps) {
  const originalContainerRef = useRef<HTMLDivElement>(null);
  const modifiedContainerRef = useRef<HTMLDivElement>(null);
  const originalEditorRef = useRef<any>(null);
  const modifiedEditorRef = useRef<any>(null);
  const originalDecorationsRef = useRef<string[]>([]);
  const modifiedDecorationsRef = useRef<string[]>([]);
  const [isInitialized, setIsInitialized] = useState(false);

  // 初始化两个编辑器
  useEffect(() => {
    if (!originalContainerRef.current || !modifiedContainerRef.current) {
      return;
    }

    console.log('[DiffEditor] Initializing editors...');
    configureMonacoWorkers();

    // 创建原始文本编辑器
    const originalEditor = monaco.editor.create(originalContainerRef.current, {
      value: originalText,
      language: 'plaintext',
      theme: 'vs',
      readOnly: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      fontSize: 14,
      lineHeight: 21,
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
    });

    // 创建修改后的文本编辑器
    const modifiedEditor = monaco.editor.create(modifiedContainerRef.current, {
      value: modifiedText,
      language: 'plaintext',
      theme: 'vs',
      readOnly: false,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      fontSize: 14,
      lineHeight: 21,
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
    });

    originalEditorRef.current = originalEditor;
    modifiedEditorRef.current = modifiedEditor;
    setIsInitialized(true);

    console.log('[DiffEditor] Editors created');

    return () => {
      originalEditor.dispose();
      modifiedEditor.dispose();
    };
  }, []);

  // 更新内容并应用差异高亮
  useEffect(() => {
    if (!originalEditorRef.current || !modifiedEditorRef.current || !isInitialized) {
      return;
    }

    console.log('[DiffEditor] Updating content and highlights...');

    // 计算差异
    const changes = diffLines(originalText, modifiedText);
    console.log('[DiffEditor] Diff changes:', changes);

    // 更新编辑器内容
    originalEditorRef.current.setValue(originalText);
    modifiedEditorRef.current.setValue(modifiedText);

    // 清除旧装饰
    originalDecorationsRef.current = originalEditorRef.current.deltaDecorations(originalDecorationsRef.current, []);
    modifiedDecorationsRef.current = modifiedEditorRef.current.deltaDecorations(modifiedDecorationsRef.current, []);

    // 应用差异高亮
    const originalDecorations: any[] = [];
    const modifiedDecorations: any[] = [];

    let originalLineNum = 1;
    let modifiedLineNum = 1;

    changes.forEach((part) => {
      // 计算 line count
      let lineCount = 1;
      if (part.count) {
        lineCount = part.count;
      } else if (part.value) {
        lineCount = part.value.split('\n').filter((line: string) => line !== '').length;
      }

      console.log('[DiffEditor] Processing part:', {
        removed: part.removed,
        added: part.added,
        lineCount,
        originalLineNum,
        modifiedLineNum
      });

      if (part.removed) {
        // 删除的行 - 在左侧编辑器高亮
        for (let i = 0; i < lineCount; i++) {
          originalDecorations.push({
            range: new (monaco as any).Range(originalLineNum + i, 1, originalLineNum + i, 1),
            options: {
              isWholeLine: true,
              className: 'line-delete',
              glyphMarginClassName: 'glyph-delete'
            }
          });
        }
        originalLineNum += lineCount;
      } else if (part.added) {
        // 新增的行 - 在右侧编辑器高亮
        for (let i = 0; i < lineCount; i++) {
          modifiedDecorations.push({
            range: new (monaco as any).Range(modifiedLineNum + i, 1, modifiedLineNum + i, 1),
            options: {
              isWholeLine: true,
              className: 'line-insert',
              glyphMarginClassName: 'glyph-insert'
            }
          });
        }
        modifiedLineNum += lineCount;
      } else {
        // 未修改的行
        originalLineNum += lineCount;
        modifiedLineNum += lineCount;
      }
    });

    // 应用装饰
    originalDecorationsRef.current = originalEditorRef.current.deltaDecorations(originalDecorationsRef.current, originalDecorations);
    modifiedDecorationsRef.current = modifiedEditorRef.current.deltaDecorations(modifiedDecorationsRef.current, modifiedDecorations);

    console.log('[DiffEditor] Applied decorations:', {
      original: originalDecorations.length,
      modified: modifiedDecorations.length
    });

  }, [originalText, modifiedText, isInitialized]);

  return (
    <div style={{
      display: 'flex',
      width: '100%',
      height: '100%',
      minHeight: '600px',
      gap: '0',
      background: 'white',
      borderRadius: '12px',
      overflow: 'hidden',
      boxShadow: '0 4px 12px rgba(0, 0, 0, 0.08)'
    }}>
      <div
        ref={originalContainerRef}
        style={{ flex: 1 }}
        className="original-editor"
        data-label="原始文件"
      />
      <div
        ref={modifiedContainerRef}
        style={{ flex: 1 }}
        className="modified-editor"
        data-label="修改后的文件"
      />
    </div>
  );
}
