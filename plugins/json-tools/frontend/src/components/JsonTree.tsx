import React from 'react';
import type { JsonPath } from '../utils/treeUtils';

interface JsonTreeProps {
  data: any;
  selectedPath: JsonPath | null;
  isExpanded: Record<string, boolean>;
  onSelectPath: (path: JsonPath) => void;
  onToggleExpand: (path: JsonPath) => void;
}

export default function JsonTree({ data, selectedPath, isExpanded, onSelectPath, onToggleExpand }: JsonTreeProps) {
  if (!data) {
    return (
      <div className="json-tree-panel">
        <div className="empty-state">
          <div className="empty-icon">📋</div>
          <div className="empty-text">输入 JSON 后在此显示树形视图</div>
        </div>
      </div>
    );
  }

  return (
    <div className="json-tree-panel">
      <div className="json-tree">
        <TreeNode
          data={data}
          path={[]}
          selectedPath={selectedPath}
          isExpanded={isExpanded}
          onSelectPath={onSelectPath}
          onToggleExpand={onToggleExpand}
        />
      </div>
    </div>
  );
}

interface TreeNodeProps {
  data: any;
  path: JsonPath;
  selectedPath: JsonPath | null;
  isExpanded: Record<string, boolean>;
  onSelectPath: (path: JsonPath) => void;
  onToggleExpand: (path: JsonPath) => void;
}

function TreeNode({ data, path, selectedPath, isExpanded, onSelectPath, onToggleExpand }: TreeNodeProps) {
  const pathStr = path.join('.');
  const isSelected = selectedPath !== null &&
    path.length === selectedPath.length &&
    path.every((p, i) => p === selectedPath[i]);

  const isContainer = Array.isArray(data) || (typeof data === 'object' && data !== null);
  const expanded = isExpanded[pathStr] ?? (path.length === 0);

  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onSelectPath(path);
  };

  const handleToggle = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (isContainer) {
      onToggleExpand(path);
    }
  };

  if (!isContainer) {
    return (
      <div
        className={`tree-node ${isSelected ? 'selected' : ''}`}
        onClick={handleClick}
      >
        <ValueNode value={data} />
      </div>
    );
  }

  const keys = Object.keys(data);

  return (
    <div className="tree-node-container">
      <div
        className={`tree-node ${isSelected ? 'selected' : ''}`}
        onClick={handleClick}
      >
        <span
          className="tree-toggle"
          onClick={handleToggle}
        >
          {expanded ? '▼' : '▶'}
        </span>
        <span className="tree-key">
          {Array.isArray(data) ? `array[${keys.length}]` : `object{${keys.length}}`}
        </span>
      </div>

      {expanded && (
        <div className="tree-children">
          {keys.map((key) => {
            const childPath = [...path, Array.isArray(data) ? parseInt(key) : key];
            const childData = data[key];

            return (
              <div key={String(key)} className="tree-child">
                {!Array.isArray(data) && (
                  <span className="tree-key">"{key}": </span>
                )}
                <TreeNode
                  data={childData}
                  path={childPath}
                  selectedPath={selectedPath}
                  isExpanded={isExpanded}
                  onSelectPath={onSelectPath}
                  onToggleExpand={onToggleExpand}
                />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ValueNode({ value }: { value: any }) {
  if (value === null) {
    return <span className="tree-null">null</span>;
  }

  if (typeof value === 'string') {
    return <span className="tree-string">"{value}"</span>;
  }

  if (typeof value === 'number') {
    return <span className="tree-number">{value}</span>;
  }

  if (typeof value === 'boolean') {
    return <span className="tree-boolean">{value.toString()}</span>;
  }

  return <span>{String(value)}</span>;
}
