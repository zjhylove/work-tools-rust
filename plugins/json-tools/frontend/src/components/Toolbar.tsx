interface ToolbarProps {
  isValid: boolean;
  onAction: (action: string) => void;
}

export default function Toolbar({ isValid, onAction }: ToolbarProps) {
  // 主操作 - 格式化 (需要有效 JSON)
  const primaryTools = [
    { id: 'format', label: '格式化', icon: '✨', requiresValidJson: true },
  ];

  // 次要操作 - 压缩、转义、去转义
  const secondaryTools = [
    { id: 'minify', label: '压缩', icon: '📦', requiresValidJson: true },
    { id: 'escape', label: '转义', icon: '🔒', requiresValidJson: false },
    { id: 'unescape', label: '去转义', icon: '🔑', requiresValidJson: false },
  ];

  // 辅助操作 - 全展开、全折叠、删除选中
  const tertiaryActions = [
    { id: 'expandAll', label: '全展开', icon: '📂' },
    { id: 'collapseAll', label: '全折叠', icon: '📁' },
    { id: 'deleteSelected', label: '删除选中', icon: '🗑️' },
  ];

  return (
    <div className="json-toolbar">
      <div className="toolbar-group toolbar-group-left">
        {/* 主操作 */}
        {primaryTools.map(tool => (
          <button
            key={tool.id}
            className="btn-tool-primary"
            disabled={tool.requiresValidJson && !isValid}
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

        {/* 次要操作 */}
        {secondaryTools.map(tool => (
          <button
            key={tool.id}
            className="btn-tool-secondary"
            disabled={tool.requiresValidJson && !isValid}
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

      <div className="toolbar-group toolbar-group-right">
        {/* 辅助操作 */}
        {tertiaryActions.map(action => (
          <button
            key={action.id}
            className="btn-tool-tertiary"
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
