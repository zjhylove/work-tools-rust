interface Props { members: Array<{ member: string; score: number }>; searchQuery: string; }

export function ZSetViewer({ members, searchQuery }: Props) {
  const filtered = searchQuery ? members.filter(m => m.member.includes(searchQuery)) : members;
  return (
    <div className="zset-editor">
      <table>
        <thead><tr><th>Member</th><th>Score</th></tr></thead>
        <tbody>
          {filtered.map(m => (
            <tr key={m.member}><td><code>{m.member}</code></td><td>{m.score}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
