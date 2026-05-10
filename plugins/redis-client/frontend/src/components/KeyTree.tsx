import { TreeNode } from '../types';

interface TreeItemProps {
  node: TreeNode;
  depth: number;
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: boolean;
  selectedSet: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
}

function TreeItem({ node, depth, selectedKey, expandedPaths, multiSelect, selectedSet, onToggle, onSelect, onMultiToggle, onContextMenu }: TreeItemProps) {
  const path = node.fullKey || node.prefix;
  const isFolder = node.fullKey === null;
  const isExpanded = expandedPaths.has(path);
  const isChecked = selectedSet.has(node.fullKey || '');

  if (isFolder) {
    return (
      <div className="tree-branch">
        <div className="tree-folder" style={{ paddingLeft: depth * 14 + 8 }}
          onClick={() => onToggle(path)}
          onContextMenu={e => onContextMenu(e, node)}>
          <span className="tree-arrow">{isExpanded ? '▾' : '▸'}</span>
          <span className="tree-folder-name">{node.name}</span>
          <span className="tree-count">{node.children.length}</span>
        </div>
        {isExpanded && node.children.map(child => (
          <TreeItem key={child.name} node={child} depth={depth + 1}
            selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={multiSelect} selectedSet={selectedSet}
            onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle}
            onContextMenu={onContextMenu} />
        ))}
      </div>
    );
  }

  return (
    <div className={`tree-leaf ${selectedKey === node.fullKey ? 'selected' : ''}`}
      style={{ paddingLeft: depth * 14 + 24 }}
      onContextMenu={e => onContextMenu(e, node)}>
      {multiSelect && (
        <input type="checkbox" className="tree-checkbox"
          checked={isChecked}
          onChange={() => node.fullKey && onMultiToggle(node.fullKey)}
          onClick={e => e.stopPropagation()} />
      )}
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
  multiSelect: boolean;
  selectedSet: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
}

export function KeyTree({ tree, selectedKey, expandedPaths, multiSelect, selectedSet, onToggle, onSelect, onMultiToggle, onContextMenu }: KeyTreeProps) {
  return (
    <div className="key-list">
      {tree.map(node => (
        <TreeItem key={node.name} node={node} depth={0}
          selectedKey={selectedKey} expandedPaths={expandedPaths}
          multiSelect={multiSelect} selectedSet={selectedSet}
          onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle}
          onContextMenu={onContextMenu} />
      ))}
    </div>
  );
}
