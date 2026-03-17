import { useState, useCallback } from 'react';
import { EditorPane } from './components/EditorPane';
import { InlineDiffView } from './components/InlineDiffView';
import { Toolbar } from './Toolbar';
import { useDiff, type DiffOptions } from './hooks/useDiff';
import { useDebounce } from './hooks/useDebounce';
import { useSyncScroll } from './hooks/useSyncScroll';
import { useDiffNavigation } from './hooks/useDiffNavigation';
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
  const [originalText, setOriginalText] = useState(
    'Line 1\nLine 2\nLine 3\nLine 4'
  );
  const [modifiedText, setModifiedText] = useState(
    'Line 1\nModified Line 2\nLine 3\nNew Line 5'
  );

  // 文件名
  const [originalFileName, setOriginalFileName] = useState<string>('');
  const [modifiedFileName, setModifiedFileName] = useState<string>('');

  // 差异选项
  const [options, setOptions] = useState<DiffOptions>({
    ignoreWhitespace: false,
    ignoreCase: false
  });

  // 视图模式
  const [viewMode, setViewMode] = useState<'side-by-side' | 'inline'>('side-by-side');

  // 错误状态
  const [error, setError] = useState<string | null>(null);

  // 防抖处理文本输入 (300ms)
  const debouncedOriginal = useDebounce(originalText, 300);
  const debouncedModified = useDebounce(modifiedText, 300);

  // 计算差异 (使用防抖后的文本)
  const diffResult = useDiff(debouncedOriginal, debouncedModified, options);

  // 同步滚动
  const {
    handleLeftScroll,
    handleRightScroll
  } = useSyncScroll({ enabled: true });

  // 差异导航
  const navigation = useDiffNavigation(
    diffResult.originalLines,
    diffResult.modifiedLines
  );

  // 文件加载处理
  const handleOriginalFileLoaded = useCallback((content: string, fileName: string) => {
    setOriginalText(content);
    setOriginalFileName(fileName);
  }, []);

  const handleModifiedFileLoaded = useCallback((content: string, fileName: string) => {
    setModifiedText(content);
    setModifiedFileName(fileName);
  }, []);

  // 文本变化处理 (立即更新,但差异计算会防抖)
  const handleOriginalChange = useCallback((content: string) => {
    setOriginalText(content);
  }, []);

  const handleModifiedChange = useCallback((content: string) => {
    setModifiedText(content);
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
      a.download = `diff-${Date.now()}.diff`;
      a.click();
      URL.revokeObjectURL(url);

      console.log('✅ 导出成功');
    } catch (err: any) {
      setError(`导出失败: ${err.message}`);
      setTimeout(() => setError(null), 3000);
    }
  }, [originalText, modifiedText]);

  // 选项切换
  const handleToggleIgnoreWhitespace = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreWhitespace: value }));
  }, []);

  const handleToggleIgnoreCase = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreCase: value }));
  }, []);

  // 视图模式切换
  const handleToggleViewMode = useCallback(() => {
    setViewMode(prev => prev === 'side-by-side' ? 'inline' : 'side-by-side');
  }, []);

  // 差异导航
  const handleNextDiff = useCallback(() => {
    navigation.goToNextAndScroll();
  }, [navigation]);

  const handlePreviousDiff = useCallback(() => {
    navigation.goToPreviousAndScroll();
  }, [navigation]);

  return (
    <div className="app">
      {error && (
        <div className="error-banner">
          ❌ {error}
          <button onClick={() => setError(null)} className="close-btn">×</button>
        </div>
      )}

      <Toolbar
        originalFileName={originalFileName || '原始文件'}
        modifiedFileName={modifiedFileName || '修改后的文件'}
        viewMode={viewMode}
        onOpenOriginal={() => {
          const input = prompt('请输入原始文件路径:');
          if (input) {
            window.pluginAPI.call('text-diff', 'load_text_file', { file_path: input })
              .then((result: any) => {
                if (result.error) throw new Error(result.error);
                handleOriginalFileLoaded(result.content, input);
              })
              .catch((err: Error) => {
                setError(`加载文件失败: ${err.message}`);
                setTimeout(() => setError(null), 3000);
              });
          }
        }}
        onOpenModified={() => {
          const input = prompt('请输入修改后的文件路径:');
          if (input) {
            window.pluginAPI.call('text-diff', 'load_text_file', { file_path: input })
              .then((result: any) => {
                if (result.error) throw new Error(result.error);
                handleModifiedFileLoaded(result.content, input);
              })
              .catch((err: Error) => {
                setError(`加载文件失败: ${err.message}`);
                setTimeout(() => setError(null), 3000);
              });
          }
        }}
        onNextDiff={handleNextDiff}
        onPreviousDiff={handlePreviousDiff}
        onExport={handleExport}
        onToggleViewMode={handleToggleViewMode}
        onToggleIgnoreWhitespace={handleToggleIgnoreWhitespace}
        onToggleIgnoreCase={handleToggleIgnoreCase}
        diffStats={diffResult.stats}
      />

      {viewMode === 'side-by-side' ? (
        <div className="editor-container">
          <EditorPane
            title={originalFileName || '原始文件'}
            content={originalText}
            diffLines={diffResult.originalLines}
            onChange={handleOriginalChange}
            onScroll={handleLeftScroll}
            placeholder="在此输入或粘贴原始文件内容..."
            className="left-pane"
          />

          <EditorPane
            title={modifiedFileName || '修改后的文件'}
            content={modifiedText}
            diffLines={diffResult.modifiedLines}
            onChange={handleModifiedChange}
            onScroll={handleRightScroll}
            placeholder="在此输入或粘贴修改后的文件内容..."
            className="right-pane"
          />
        </div>
      ) : (
        <div className="inline-container">
          <InlineDiffView
            originalLines={diffResult.originalLines}
            modifiedLines={diffResult.modifiedLines}
          />
        </div>
      )}
    </div>
  );
}

export default App;
