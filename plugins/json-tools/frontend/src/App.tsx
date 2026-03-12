import { useState, useEffect, useMemo } from 'react';
import './App.css';
import Toolbar from './components/Toolbar';
import JsonEditor from './components/JsonEditor';
import JsonTree from './components/JsonTree';
import { validateJson, formatJson, minifyJson, escapeJson, unescapeJson, type ValidationError } from './utils/jsonUtils';
import { deleteByPath, expandAll, type JsonPath } from './utils/treeUtils';

function App() {
  const [jsonText, setJsonText] = useState<string>('{\n  \n}');
  const [parsedData, setParsedData] = useState<any>(null);
  const [error, setError] = useState<ValidationError | null>(null);
  const [selectedPath, setSelectedPath] = useState<JsonPath | null>(null);
  const [isExpanded, setIsExpanded] = useState<Record<string, boolean>>({ 'root': true });
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [isFormatted, setIsFormatted] = useState<boolean>(false);

  // 计算节点数量
  const nodeCount = useMemo(() => {
    if (!parsedData) return 0;
    let count = 0;

    const traverse = (obj: any) => {
      count++;
      if (Array.isArray(obj)) {
        obj.forEach(traverse);
      } else if (typeof obj === 'object' && obj !== null) {
        Object.values(obj).forEach(traverse);
      }
    };

    traverse(parsedData);
    return count;
  }, [parsedData]);

  // 格式化数据大小
  const dataSize = useMemo(() => {
    const bytes = new Blob([jsonText]).size;
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }, [jsonText]);

  // 实时验证 JSON
  useEffect(() => {
    const validation = validateJson(jsonText);
    setError(validation);

    if (validation.valid) {
      try {
        const parsed = JSON.parse(jsonText);
        setParsedData(parsed);
      } catch (e) {
        // 解析失败,保持原状
      }
    } else {
      setParsedData(null);
    }
  }, [jsonText]);

  // 处理工具栏操作
  const handleToolAction = async (action: string) => {
    try {
      let result: string;

      switch (action) {
        case 'format':
          result = formatJson(jsonText);
          setIsFormatted(true);
          break;
        case 'minify':
          result = minifyJson(jsonText);
          setIsFormatted(false);
          break;
        case 'escape':
          result = escapeJson(jsonText);
          setIsFormatted(false);
          break;
        case 'unescape':
          result = unescapeJson(jsonText);
          setIsFormatted(true);
          break;
        case 'expandAll':
          setIsExpanded(expandAll(parsedData));
          return;
        case 'collapseAll':
          setIsExpanded({ 'root': true });
          return;
        case 'deleteSelected':
          if (selectedPath) {
            const newData = deleteByPath(parsedData, selectedPath);
            const newText = JSON.stringify(newData, null, 2);
            setParsedData(newData);
            setJsonText(newText);
            setSelectedPath(null);
          }
          return;
        default:
          return;
      }

      setJsonText(result);
      setSuccessMessage('操作成功');
      setTimeout(() => setSuccessMessage(''), 2000);
    } catch (e) {
      setError({
        valid: false,
        error: (e as Error).message
      });
    }
  };

  return (
    <div className="json-tools">
      <Toolbar
        isValid={error?.valid ?? false}
        onAction={handleToolAction}
      />

      <div className="json-workspace">
        <div className="json-editor-panel-wrapper">
          <JsonEditor
            value={jsonText}
            onChange={setJsonText}
            error={error}
          />
          {/* 错误提示 - 在编辑器下方 */}
          {error && !error.valid && (
            <div className="json-error">
              <div className="error-message">
                <div className="error-title">
                  ⚠️ {error.error}
                </div>
                <div className="error-details">
                  {(error.line || error.column) && (
                    <span className="error-location">
                      {error.line && `第 ${error.line} 行`}
                      {error.line && error.column && ' '}
                      {error.column && `第 ${error.column} 列`}
                    </span>
                  )}
                  {error.suggestion && (
                    <span className="error-suggestion">
                      💡 {error.suggestion}
                    </span>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
        <JsonTree
          data={parsedData}
          selectedPath={selectedPath}
          isExpanded={isExpanded}
          onSelectPath={setSelectedPath}
          onToggleExpand={(path) => {
            const pathStr = path.join('.');
            setIsExpanded(prev => ({
              ...prev,
              [pathStr]: !prev[pathStr]
            }));
          }}
        />
      </div>

      {/* 空状态提示 */}
      {(!parsedData && jsonText === '{\n  \n}') && (
        <div className="empty-state">
          <div className="empty-icon">📝</div>
          <div className="empty-title">开始使用 JSON 工具</div>
          <div className="empty-description">
            在左侧输入 JSON 文本,或点击上方"格式化"按钮查看示例
          </div>
        </div>
      )}

      {/* 成功提示 - 固定在顶部 */}
      {successMessage && (
        <div className="json-success">
          ✓ {successMessage}
        </div>
      )}

      {/* 底部状态栏 */}
      <div className="json-statusbar">
        <div className="statusbar-left">
          <div className="status-item">
            <span>📊</span>
            <span>共 {nodeCount} 个节点</span>
          </div>
          {isFormatted && (
            <div className="status-item">
              <span>✨</span>
              <span>已格式化</span>
            </div>
          )}
        </div>
        <div className="statusbar-right">
          <div className="status-item">
            <span>📦</span>
            <span>大小: {dataSize}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
