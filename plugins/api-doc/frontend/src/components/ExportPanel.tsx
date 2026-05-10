import { ApiInfo } from '../types'

interface Props {
  apis: ApiInfo[]
  exportFormats: string[]
  outputDir: string
  loading: boolean
  onFormatChange: (formats: string[]) => void
  onOutputDirChange: (dir: string) => void
  onOpenOutputDir: () => void
  onExport: () => void
}

export default function ExportPanel({
  apis, exportFormats, outputDir, loading,
  onFormatChange, onOutputDirChange, onOpenOutputDir, onExport,
}: Props) {
  if (apis.length === 0) return null

  return (
    <div className="export-panel">
      <div className="export-panel-header">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
        <span>导出设置</span>
        <span className="export-count">{apis.length} 个 API</span>
      </div>

      <div className="export-options">
        <div className="format-options">
          {['markdown', 'html'].map(fmt => (
            <label key={fmt} className={`format-chip ${exportFormats.includes(fmt) ? 'format-chip--active' : ''}`}>
              <input
                type="checkbox"
                checked={exportFormats.includes(fmt)}
                onChange={e => {
                  if (e.target.checked) onFormatChange([...exportFormats, fmt])
                  else onFormatChange(exportFormats.filter(f => f !== fmt))
                }}
              />
              <span>{fmt.toUpperCase()}</span>
            </label>
          ))}
        </div>

        <div className="input-row">
          <input
            type="text"
            className="form-input form-input--sm"
            value={outputDir}
            onChange={e => onOutputDirChange(e.target.value)}
            placeholder="选择输出目录"
          />
          <button onClick={onOpenOutputDir} className="btn btn--outline btn--sm">浏览</button>
        </div>

        <button onClick={onExport} className="btn btn--primary btn--block" disabled={loading}>
          {loading ? (
            <><span className="spinner" /> 导出中...</>
          ) : `导出 ${apis.length} 个 API`}
        </button>
      </div>
    </div>
  )
}
