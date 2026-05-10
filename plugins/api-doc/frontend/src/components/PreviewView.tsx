interface Props {
  outputFiles: string[]
  onBack: () => void
  onRestart: () => void
}

export default function PreviewView({ outputFiles, onBack, onRestart }: Props) {
  return (
    <div className="view-container view-container--centered">
      <div className="card card--preview">
        <div className="preview-success">
          <div className="success-icon">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
              <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
          </div>
          <h2>导出完成</h2>
          <p className="preview-subtitle">已生成 {outputFiles.length} 个文件</p>
        </div>

        <div className="export-results">
          {outputFiles.map((f, i) => (
            <div key={i} className="export-result-item">
              <span className="result-icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </span>
              <span className="result-file">{f}</span>
            </div>
          ))}
        </div>

        <div className="preview-actions">
          <button onClick={onBack} className="btn btn--outline">
            返回选择
          </button>
          <button onClick={onRestart} className="btn btn--primary">
            重新开始
          </button>
        </div>
      </div>
    </div>
  )
}
