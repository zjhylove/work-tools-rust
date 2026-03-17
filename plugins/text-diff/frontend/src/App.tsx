import { useState, useCallback, useEffect } from 'react';
import { EditorPane } from './components/EditorPane';
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

  // 差异统计
  const [diffStats, setDiffStats] = useState({
    additions: 0,
    deletions: 0,
    modifications: 0
  });

  // 错误状态
  const [error, setError] = useState<string | null>(null);

  // 计算差异统计 (简化版,暂时用前端计算)
  useEffect(() => {
    const calculateStats = () => {
      const origLines = originalText.split('\n');
      const modLines = modifiedText.split('\n');

      // 简单的差异检测 (实际应该使用 diff 库)
      let additions = 0;
      let deletions = 0;

      modLines.forEach(line => {
        if (!origLines.includes(line)) additions++;
      });

      origLines.forEach(line => {
        if (!modLines.includes(line)) deletions++;
      });

      const modifications = Math.min(additions, deletions);
      additions -= modifications;
      deletions -= modifications;

      setDiffStats({ additions, deletions, modifications });
    };

    calculateStats();
  }, [originalText, modifiedText]);

  // 文件加载处理
  const handleOriginalFileLoaded = useCallback((content: string, fileName: string) => {
    setOriginalText(content);
    setOriginalFileName(fileName);
  }, []);

  const handleModifiedFileLoaded = useCallback((content: string, fileName: string) => {
    setModifiedText(content);
    setModifiedFileName(fileName);
  }, []);

  // 文本变化处理
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

  // 选项切换 (暂时为空,后续实现)
  const handleToggleIgnoreWhitespace = useCallback((value: boolean) => {
    console.log('忽略空白:', value);
    // TODO: 实现忽略空白逻辑
  }, []);

  const handleToggleIgnoreCase = useCallback((value: boolean) => {
    console.log('忽略大小写:', value);
    // TODO: 实现忽略大小写逻辑
  }, []);

  // 占位符函数 (差异导航)
  const handleNextDiff = useCallback(() => {
    console.log('下一个差异');
  }, []);

  const handlePreviousDiff = useCallback(() => {
    console.log('上一个差异');
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
        originalFileName={originalFileName || '未选择文件'}
        modifiedFileName={modifiedFileName || '未选择文件'}
        onOpenOriginal={() => {
          // 使用 prompt 作为临时方案
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
        onToggleIgnoreWhitespace={handleToggleIgnoreWhitespace}
        onToggleIgnoreCase={handleToggleIgnoreCase}
        diffStats={diffStats}
      />

      <div className="editor-container">
        <EditorPane
          title={originalFileName || '原始文件'}
          content={originalText}
          onChange={handleOriginalChange}
          placeholder="在此输入或粘贴原始文件内容..."
          className="left-pane"
        />

        <EditorPane
          title={modifiedFileName || '修改后的文件'}
          content={modifiedText}
          onChange={handleModifiedChange}
          placeholder="在此输入或粘贴修改后的文件内容..."
          className="right-pane"
        />
      </div>
    </div>
  );
}

export default App;
