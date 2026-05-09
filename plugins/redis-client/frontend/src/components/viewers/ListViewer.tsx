interface Props { items: string[]; searchQuery: string; }

export function ListViewer({ items, searchQuery }: Props) {
  const filtered = searchQuery ? items.filter(i => i.includes(searchQuery)) : items;
  return (
    <div className="list-editor">
      <ol>{filtered.map((item, i) => <li key={i}><code>{item}</code></li>)}</ol>
      {searchQuery && <div className="search-info">{filtered.length} / {items.length} 条匹配</div>}
    </div>
  );
}
