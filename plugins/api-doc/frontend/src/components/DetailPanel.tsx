import { ApiInfo } from '../types'
import ApiCard from './ApiCard'
import ExportPanel from './ExportPanel'

interface Props {
  apis: ApiInfo[]
  expandedApis: Set<string>
  searchFilter: string
  onToggleApi: (path: string) => void
  exportFormats: string[]
  outputDir: string
  loading: boolean
  onFormatChange: (formats: string[]) => void
  onOutputDirChange: (dir: string) => void
  onOpenOutputDir: () => void
  onExport: () => void
  onSearchChange: (v: string) => void
}

// 判断 API 是否匹配搜索条件
function apiMatches(api: ApiInfo, query: string): boolean {
  if (!query) return true
  const q = query.toLowerCase()
  return (
    api.api_name.toLowerCase().includes(q) ||
    api.full_path.toLowerCase().includes(q) ||
    api.http_method.toLowerCase().includes(q) ||
    api.service_name.toLowerCase().includes(q) ||
    api.business_module.toLowerCase().includes(q)
  )
}

export default function DetailPanel({
  apis, expandedApis, searchFilter, onToggleApi,
  exportFormats, outputDir, loading,
  onFormatChange, onOutputDirChange, onOpenOutputDir, onExport,
  onSearchChange,
}: Props) {
  const query = searchFilter.trim()

  // 过滤匹配的 API
  const filteredApis = apis.filter(api => apiMatches(api, query))

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
            <div className="panel-heading-actions">
              <span className="panel-heading-count">
                {query ? `${filteredApis.length}/${apis.length}` : `${apis.length}`} 个 API
              </span>
              <div className="search-box search-box--compact">
                <svg className="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
                <input
                  type="text"
                  className="search-input search-input--sm"
                  placeholder="搜索 API..."
                  value={searchFilter}
                  onChange={e => onSearchChange(e.target.value)}
                />
              </div>
            </div>
          </div>
          <div className="api-list">
            {filteredApis.length === 0 ? (
              <div className="panel-empty">
                <p>未找到匹配的 API</p>
                <p className="panel-empty-sub">请尝试其他搜索关键词</p>
              </div>
            ) : (
              filteredApis.map(api => (
                <ApiCard
                  key={api.full_path}
                  api={api}
                  isExpanded={expandedApis.has(api.full_path)}
                  onToggle={() => onToggleApi(api.full_path)}
                  searchQuery={query}
                />
              ))
            )}
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
