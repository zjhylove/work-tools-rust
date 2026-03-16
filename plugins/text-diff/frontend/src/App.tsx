import { useState, useCallback, useEffect, useRef } from 'react';
import { DiffEditor } from './DiffEditor';
import { Toolbar } from './Toolbar';
import './App.css';
import './Toolbar.css';

// 声明 Tauri API
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params: any) => Promise<any>;
    };
  }
}

function App() {
  const [originalText, setOriginalText] = useState(
    'Line 1\nLine 2\nLine 3\nLine 4'
  );
  const [modifiedText, setModifiedText] = useState(
    'Line 1\nModified Line 2\nLine 3\nNew Line 5'
  );
  const [options, setOptions] = useState({
    ignoreWhitespace: false,
    ignoreCase: false
  });
  const [diffStats] = useState({
    additions: 0,
    deletions: 0,
    modifications: 0
  });
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<any | null>(null);

  // 文件打开处理
  const handleFileOpen = useCallback(async (side: 'left' | 'right') => {
    console.log('[TextDiff] handleFileOpen called with side:', side);
    setError(null);

    try {
      // 直接使用 prompt 输入文件路径 (iframe 环境中无法使用 Tauri dialog)
      console.log('[TextDiff] Showing prompt...');
      const userInput = prompt('请输入文件路径:\n\n例如: /tmp/test-original.txt');
      console.log('[TextDiff] User input:', userInput);

      if (!userInput) {
        // 用户取消了选择
        console.log('[TextDiff] User cancelled');
        return;
      }

      // 调用后端加载文件
      console.log('[TextDiff] Calling pluginAPI...');
      const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
        file_path: userInput
      });
      console.log('[TextDiff] PluginAPI result:', result);

      if (result.error) {
        console.error('[TextDiff] Error from plugin:', result.error);
        setError(result.error);
        return;
      }

      if (side === 'left') {
        console.log('[TextDiff] Setting original text');
        setOriginalText(result.content);
      } else {
        console.log('[TextDiff] Setting modified text');
        setModifiedText(result.content);
      }
    } catch (err: any) {
      console.error('[TextDiff] Exception:', err);
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

  // 计算差异统计 (暂时禁用,避免阻塞)
  // TODO: 修复后端 count_diff 方法的性能问题后再启用
  /*
  useEffect(() => {
    if (!originalText || !modifiedText) {
      setDiffStats({ additions: 0, deletions: 0, modifications: 0 });
      return;
    }

    const calculateStats = async () => {
      try {
        console.log('[TextDiff] Calculating diff stats...');
        const stats = await window.pluginAPI.call('text-diff', 'count_diff', {
          original: originalText,
          modified: modifiedText
        });
        console.log('[TextDiff] Diff stats:', stats);
        setDiffStats(stats);
      } catch (err: any) {
        console.error('[TextDiff] Count diff error:', err);
        // 失败时使用默认值
        setDiffStats({ additions: 0, deletions: 0, modifications: 0 });
      }
    };

    calculateStats();
  }, [originalText, modifiedText]);
  */

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
