interface ToolbarProps {
  isValid: boolean;
  onAction: (action: string) => void;
}

export default function Toolbar({ isValid, onAction }: ToolbarProps) {
  const tools = [
    { id: 'format', label: '格式化', icon: '✨' },
    { id: 'minify', label: '压缩', icon: '📦' },
    { id: 'escape', label: '转义', icon: '🔒' },
    { id: 'unescape', label: '去转义', icon: '🔑' },
  ];

  const treeActions = [
    { id: 'expandAll', label: '全展开', icon: '📂' },
    { id: 'collapseAll', label: '全折叠', icon: '📁' },
    { id: 'deleteSelected', label: '删除选中', icon: '🗑️' },
  ];

  return (
    <div className="json-toolbar">
      <div className="toolbar-group">
        {tools.map(tool => (
          <button
            key={tool.id}
            className="btn-tool"
            disabled={!isValid}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onAction(tool.id);
            }}
            title={tool.label}
          >
            {tool.icon} {tool.label}
          </button>
        ))}
      </div>

      <div className="toolbar-group">
        {treeActions.map(action => (
          <button
            key={action.id}
            className="btn-tool"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onAction(action.id);
            }}
            title={action.label}
          >
            {action.icon} {action.label}
          </button>
        ))}
      </div>
    </div>
  );
}
