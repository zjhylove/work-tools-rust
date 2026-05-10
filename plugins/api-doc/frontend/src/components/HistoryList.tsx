import { ExportHistory } from '../types'

interface Props {
  history: ExportHistory[]
}

export default function HistoryList({ history }: Props) {
  if (history.length === 0) return null

  return (
    <div className="history-section">
      <div className="section-label">最近导出</div>
      <div className="history-list">
        {history.slice(-5).reverse().map(h => (
          <div key={h.id} className="history-item">
            <div className="history-item-main">
              <span className="history-name">{h.service_name}</span>
              <span className="history-count">{h.api_count} APIs</span>
            </div>
            <span className="history-time">{new Date(h.exported_at).toLocaleString()}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
