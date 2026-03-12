import { useState, useCallback, useEffect, useRef } from 'react';
import { DiffEditor } from './DiffEditor';
import { Toolbar } from './Toolbar';
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
  const [originalText, setOriginalText] = useState('');
  const [modifiedText, setModifiedText] = useState('');
  const [options, setOptions] = useState({
    ignoreWhitespace: false,
    ignoreCase: false
  });
  const [diffStats, setDiffStats] = useState({
    additions: 0,
    deletions: 0,
    modifications: 0
  });
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<any | null>(null);

  // 文件打开处理
  const handleFileOpen = useCallback(async (side: 'left' | 'right') => {
    setError(null);

    try {
      // 临时测试路径 - 实际应该使用 Tauri 文件对话框
      const testFilePath = '/tmp/test.txt';
      const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
        file_path: testFilePath
      });

      if (result.error) {
        setError(result.error);
        return;
      }

      if (side === 'left') {
        setOriginalText(result.content);
      } else {
        setModifiedText(result.content);
      }
    } catch (err: any) {
      setError(`加载文件失败: ${err.message}`);
    }
  }, []);

  // 差异导航
  const handleNextDiff = useCallback(() => {
    if (!editorRef.current) return;
    editorRef.current.goToDiff('next');
  }, []);

  const handlePreviousDiff = useCallback(() => {
    if (!editorRef.current) return;
    editorRef.current.goToDiff('previous');
  }, []);

  // 导出差异
  const handleExport = useCallback(async () => {
    try {
      const result = await window.pluginAPI.call('text-diff', 'export_diff', {
        original: originalText,
        modified: modifiedText,
        filename: 'changes.diff'
      });

      // 创建下载链接
      const blob = new Blob([result.diff], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'changes.diff';
      a.click();
      URL.revokeObjectURL(url);
    } catch (err: any) {
      setError(`导出失败: ${err.message}`);
    }
  }, [originalText, modifiedText]);

  // 选项切换
  const handleToggleIgnoreWhitespace = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreWhitespace: value }));
  }, []);

  const handleToggleIgnoreCase = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreCase: value }));
  }, []);

  // 计算差异统计
  useEffect(() => {
    if (!originalText || !modifiedText) {
      setDiffStats({ additions: 0, deletions: 0, modifications: 0 });
      return;
    }

    const calculateStats = async () => {
      try {
        const stats = await window.pluginAPI.call('text-diff', 'count_diff', {
          original: originalText,
          modified: modifiedText
        });
        setDiffStats(stats);
      } catch (err: any) {
        console.error('Count diff error:', err);
      }
    };

    calculateStats();
  }, [originalText, modifiedText]);

  // 键盘快捷键
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // F8: 下一个差异
      if (e.key === 'F8' && !e.shiftKey) {
        e.preventDefault();
        handleNextDiff();
      }
      // Shift + F8: 上一个差异
      if (e.key === 'F8' && e.shiftKey) {
        e.preventDefault();
        handlePreviousDiff();
      }
      // Ctrl+O: 打开左侧文件
      if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
        e.preventDefault();
        handleFileOpen('left');
      }
      // Ctrl+Shift+O: 打开右侧文件
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'O') {
        e.preventDefault();
        handleFileOpen('right');
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleNextDiff, handlePreviousDiff, handleFileOpen]);

  return (
    <div className="app">
      {error && (
        <div className="error-banner">
          ❌ {error}
          <button onClick={() => setError(null)} className="close-btn">×</button>
        </div>
      )}

      <Toolbar
        onOpenLeft={() => handleFileOpen('left')}
        onOpenRight={() => handleFileOpen('right')}
        onNextDiff={handleNextDiff}
        onPreviousDiff={handlePreviousDiff}
        onExport={handleExport}
        onToggleIgnoreWhitespace={handleToggleIgnoreWhitespace}
        onToggleIgnoreCase={handleToggleIgnoreCase}
        diffStats={diffStats}
      />

      <DiffEditor
        originalText={originalText}
        modifiedText={modifiedText}
        options={options}
        onEditorReady={(editor) => {
          editorRef.current = editor;
        }}
      />
    </div>
  );
}

export default App;
