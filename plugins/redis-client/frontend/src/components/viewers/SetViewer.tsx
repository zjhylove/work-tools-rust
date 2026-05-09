interface Props { members: string[]; searchQuery: string; }

export function SetViewer({ members, searchQuery }: Props) {
  const filtered = searchQuery ? members.filter(m => m.includes(searchQuery)) : members;
  return (
    <div className="set-editor">
      {filtered.map(m => <span key={m} className="member-tag">{m}</span>)}
      {searchQuery && <div className="search-info">{filtered.length} / {members.length} 条匹配</div>}
    </div>
  );
}
