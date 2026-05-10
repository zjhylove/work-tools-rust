import { KeyInfo, TreeNode } from '../types';

export function buildTree(keys: KeyInfo[]): TreeNode[] {
  const root: TreeNode = { name: '', fullKey: null, prefix: '', children: [] };
  for (const k of keys) {
    const parts = k.key.split(':');
    let node = root;
    let parentPrefix = '';
    for (let i = 0; i < parts.length; i++) {
      const isLast = i === parts.length - 1;
      const segPrefix = parentPrefix + parts[i] + ':';
      let child = node.children.find(c => c.name === parts[i]);
      if (!child) {
        child = { name: parts[i], fullKey: isLast ? k.key : null, prefix: isLast ? k.key : segPrefix, children: [] };
        if (isLast) child.keyInfo = k;
        node.children.push(child);
      } else if (isLast) {
        child.fullKey = k.key;
        child.keyInfo = k;
        child.prefix = k.key;
      }
      parentPrefix = segPrefix;
      node = child;
    }
  }
  return root.children;
}
