import { TreeNode } from '../types';

interface TreeItemProps {
  node: TreeNode;
  depth: number;
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
}

function TreeItem({ node, depth, selectedKey, expandedPaths, multiSelect, onToggle, onSelect, onMultiToggle }: TreeItemProps) {
  const path = node.fullKey || node.name;
  const isFolder = node.fullKey === null;
  const isExpanded = expandedPaths.has(path);
  const isSelected = multiSelect.has(node.fullKey || '');

  if (isFolder) {
    return (
      <div className="tree-branch">
        <div className="tree-folder" style={{ paddingLeft: depth * 14 + 8 }} onClick={() => onToggle(path)}>
          <span className="tree-arrow">{isExpanded ? '▾' : '▸'}</span>
          <span className="tree-folder-name">{node.name}</span>
        </div>
        {isExpanded && node.children.map(child => (
          <TreeItem key={child.name} node={child} depth={depth + 1}
            selectedKey={selectedKey} expandedPaths={expandedPaths} multiSelect={multiSelect}
            onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
        ))}
      </div>
    );
  }

  return (
    <div className={`tree-leaf ${selectedKey === node.fullKey ? 'selected' : ''}`}
      style={{ paddingLeft: depth * 14 + 24 }}>
      <input type="checkbox" className="tree-checkbox"
        checked={isSelected}
        onChange={() => node.fullKey && onMultiToggle(node.fullKey)}
        onClick={e => e.stopPropagation()} />
      <div className="tree-leaf-main" onClick={() => node.fullKey && onSelect(node.fullKey)}>
        {node.keyInfo && (
          <span className="key-type-badge" data-type={node.keyInfo.type}>{node.keyInfo.type}</span>
        )}
        <span className="tree-leaf-name">{node.name}</span>
        {node.keyInfo && node.keyInfo.ttl > 0 && (
          <span className="key-ttl">{node.keyInfo.ttl}s</span>
        )}
      </div>
    </div>
  );
}

interface KeyTreeProps {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
}

export function KeyTree({ tree, selectedKey, expandedPaths, multiSelect, onToggle, onSelect, onMultiToggle }: KeyTreeProps) {
  return (
    <div className="key-list">
      {tree.map(node => (
        <TreeItem key={node.name} node={node} depth={0}
          selectedKey={selectedKey} expandedPaths={expandedPaths} multiSelect={multiSelect}
          onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
      ))}
    </div>
  );
}
