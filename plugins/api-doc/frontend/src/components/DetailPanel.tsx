import { ApiInfo } from '../types'
import ApiCard from './ApiCard'
import ExportPanel from './ExportPanel'

interface Props {
  apis: ApiInfo[]
  expandedApis: Set<string>
  onToggleApi: (path: string) => void
  exportFormats: string[]
  outputDir: string
  loading: boolean
  onFormatChange: (formats: string[]) => void
  onOutputDirChange: (dir: string) => void
  onOpenOutputDir: () => void
  onExport: () => void
}

export default function DetailPanel({
  apis, expandedApis, onToggleApi,
  exportFormats, outputDir, loading,
  onFormatChange, onOutputDirChange, onOpenOutputDir, onExport,
}: Props) {
  return (
    <div className="panel panel--right">
      {apis.length === 0 ? (
        <div className="panel-empty">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
          </svg>
          <p>选择左侧的 Controller 并点击「解析」</p>
          <p className="panel-empty-sub">解析后的 API 详情将显示在这里</p>
        </div>
      ) : (
        <>
          <div className="panel-heading">
            <span className="panel-heading-text">解析结果</span>
            <span className="panel-heading-count">{apis.length} 个 API</span>
          </div>
          <div className="api-list">
            {apis.map(api => (
              <ApiCard
                key={api.full_path}
                api={api}
                isExpanded={expandedApis.has(api.full_path)}
                onToggle={() => onToggleApi(api.full_path)}
              />
            ))}
          </div>
          <ExportPanel
            apis={apis}
            exportFormats={exportFormats}
            outputDir={outputDir}
            loading={loading}
            onFormatChange={onFormatChange}
            onOutputDirChange={onOutputDirChange}
            onOpenOutputDir={onOpenOutputDir}
            onExport={onExport}
          />
        </>
      )}
    </div>
  )
}
