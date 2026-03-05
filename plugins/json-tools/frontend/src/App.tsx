import { useState, useEffect } from 'react';
import './App.css';
import Toolbar from './components/Toolbar';
import JsonEditor from './components/JsonEditor';
import JsonTree from './components/JsonTree';
import { validateJson, formatJson, minifyJson, escapeJson, unescapeJson } from './utils/jsonUtils';
import { deleteByPath, expandAll, type JsonPath } from './utils/treeUtils';

interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
}

function App() {
  const [jsonText, setJsonText] = useState<string>('{\n  \n}');
  const [parsedData, setParsedData] = useState<any>(null);
  const [error, setError] = useState<ValidationError | null>(null);
  const [selectedPath, setSelectedPath] = useState<JsonPath | null>(null);
  const [isExpanded, setIsExpanded] = useState<Record<string, boolean>>({ 'root': true });
  const [successMessage, setSuccessMessage] = useState<string>('');

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
          break;
        case 'minify':
          result = minifyJson(jsonText);
          break;
        case 'escape':
          result = escapeJson(jsonText);
          break;
        case 'unescape':
          result = unescapeJson(jsonText);
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

      {successMessage && (
        <div className="json-success">
          ✓ {successMessage}
        </div>
      )}

      <div className="json-workspace">
        <JsonEditor
          value={jsonText}
          onChange={setJsonText}
          error={error}
        />
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

      {error && !error.valid && (
        <div className="json-error">
          ⚠️ {error.error}
        </div>
      )}
    </div>
  );
}

export default App;
