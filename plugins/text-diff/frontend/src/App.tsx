import { useState, useCallback, useRef, type ChangeEvent } from 'react';
import { EditorPane } from './components/EditorPane';
import { Toolbar } from './Toolbar';
import { useDiff } from './hooks/useDiff';
import './App.css';

// 声明 Tauri API
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params: any) => Promise<any>;
    };
  }
}

function App() {
  // 文本内容
  const [originalText, setOriginalText] = useState('');
  const [modifiedText, setModifiedText] = useState('');

  // 文件名
  const [originalFileName, setOriginalFileName] = useState<string>('');
  const [modifiedFileName, setModifiedFileName] = useState<string>('');

  // 错误状态
  const [error, setError] = useState<string | null>(null);

  // 文件输入 refs
  const originalFileInputRef = useRef<HTMLInputElement>(null);
  const modifiedFileInputRef = useRef<HTMLInputElement>(null);

  // 计算差异 (useDiff内部已有防抖，无需在外层再加)
  const diffResult = useDiff(originalText, modifiedText, {
    ignoreWhitespace: false,
    ignoreCase: false
  });

  // 使用原生文件选择器 (替代 prompt())
  const handleOriginalFileChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      setOriginalText(content);
      setOriginalFileName(file.name);
    };
    reader.onerror = () => {
      setError(`读取文件失败: ${file.name}`);
      setTimeout(() => setError(null), 3000);
    };
    reader.readAsText(file);
  }, []);

  const handleModifiedFileChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      setModifiedText(content);
      setModifiedFileName(file.name);
    };
    reader.onerror = () => {
      setError(`读取文件失败: ${file.name}`);
      setTimeout(() => setError(null), 3000);
    };
    reader.readAsText(file);
  }, []);

  // 文本变化处理
  const handleOriginalChange = useCallback((content: string) => {
    setOriginalText(content);
  }, []);

  const handleModifiedChange = useCallback((content: string) => {
    setModifiedText(content);
  }, []);

  return (
    <div className="app">
      {error && (
        <div className="error-banner">
          ❌ {error}
          <button onClick={() => setError(null)} className="close-btn">×</button>
        </div>
      )}

      <Toolbar
        originalFileName={originalFileName}
        modifiedFileName={modifiedFileName}
        onOpenOriginal={() => originalFileInputRef.current?.click()}
        onOpenModified={() => modifiedFileInputRef.current?.click()}
      />

      {/* 隐藏的文件输入 */}
      <input
        ref={originalFileInputRef}
        type="file"
        accept=".txt,.md,.js,.ts,.jsx,.tsx,.json,.csv,.html,.css,.py,.rs,.go,.java,.c,.cpp,.h,.hpp,.sh,.bat,.xml,.yaml,.yml,.toml"
        onChange={handleOriginalFileChange}
        style={{ display: 'none' }}
      />
      <input
        ref={modifiedFileInputRef}
        type="file"
        accept=".txt,.md,.js,.ts,.jsx,.tsx,.json,.csv,.html,.css,.py,.rs,.go,.java,.c,.cpp,.h,.hpp,.sh,.bat,.xml,.yaml,.yml,.toml"
        onChange={handleModifiedFileChange}
        style={{ display: 'none' }}
      />

      {/* 并排编辑器 */}
      <div className="editor-container">
        <EditorPane
          title="原始文件"
          content={originalText}
          diffLines={diffResult.originalLines}
          onChange={handleOriginalChange}
          placeholder="在此输入或粘贴原始文件内容..."
          className="left-pane"
        />

        <EditorPane
          title="修改后的文件"
          content={modifiedText}
          diffLines={diffResult.modifiedLines}
          onChange={handleModifiedChange}
          placeholder="在此输入或粘贴修改后的文件内容..."
          className="right-pane"
        />
      </div>
    </div>
  );
}

export default App;
