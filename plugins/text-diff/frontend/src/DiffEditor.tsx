import { useEffect, useRef } from 'react';
import * as monaco from 'monaco-editor';
import './DiffEditor.css';

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
  options,
  onEditorReady
}: DiffEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<any>(null);

  // 初始化 Monaco Diff Editor
  useEffect(() => {
    if (!containerRef.current) return;

    editorRef.current = (monaco as any).editor.createDiffEditor(containerRef.current, {
      enableSplitViewResizing: true,
      renderSideBySide: true,
      ignoreTrimWhitespace: options.ignoreWhitespace,
      readOnly: false,
      automaticLayout: true,
      theme: 'vs-dark',
      diffWordWrap: 'on'
    });

    onEditorReady?.(editorRef.current);

    return () => {
      if (editorRef.current) {
        editorRef.current.dispose();
        editorRef.current = null;
      }
    };
  }, []);

  // 更新文本模型
  useEffect(() => {
    if (!editorRef.current) return;

    const originalModel = (monaco as any).editor.createModel(originalText, 'plaintext');
    const modifiedModel = (monaco as any).editor.createModel(modifiedText, 'plaintext');

    editorRef.current.setModel({
      original: originalModel,
      modified: modifiedModel
    });

    return () => {
      originalModel.dispose();
      modifiedModel.dispose();
    };
  }, [originalText, modifiedText]);

  // 更新选项
  useEffect(() => {
    if (!editorRef.current) return;

    editorRef.current.updateOptions({
      ignoreTrimWhitespace: options.ignoreWhitespace
    });
  }, [options.ignoreWhitespace]);

  return <div ref={containerRef} className="diff-editor-container" />;
}
