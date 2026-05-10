import { ControllerInfo, ApiInfo } from '../types'
import ControllerPanel from './ControllerPanel'
import DetailPanel from './DetailPanel'

interface Props {
  controllers: ControllerInfo[]
  selectedMethods: Set<string>
  expandedClasses: Set<string>
  searchFilter: string
  apis: ApiInfo[]
  expandedApis: Set<string>
  loading: boolean
  exportFormats: string[]
  outputDir: string
  onBack: () => void
  onToggleMethod: (key: string) => void
  onToggleClass: (className: string) => void
  onToggleExpand: (cn: string) => void
  onSelectAll: () => void
  onDeselectAll: () => void
  onSearchChange: (v: string) => void
  onParse: () => void
  onToggleApi: (path: string) => void
  onFormatChange: (formats: string[]) => void
  onOutputDirChange: (dir: string) => void
  onOpenOutputDir: () => void
  onExport: () => void
}

export default function SelectView({
  controllers, selectedMethods, expandedClasses, searchFilter,
  apis, expandedApis, loading, exportFormats, outputDir,
  onBack, onToggleMethod, onToggleClass, onToggleExpand,
  onSelectAll, onDeselectAll, onSearchChange, onParse, onToggleApi,
  onFormatChange, onOutputDirChange, onOpenOutputDir, onExport,
}: Props) {
  return (
    <div className="view-container view-container--split">
      <div className="split-back">
        <button onClick={onBack} className="btn btn--ghost">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
          返回配置
        </button>
      </div>
      <div className="split-layout">
        <ControllerPanel
          controllers={controllers}
          selectedMethods={selectedMethods}
          expandedClasses={expandedClasses}
          searchFilter={searchFilter}
          loading={loading}
          onToggleMethod={onToggleMethod}
          onToggleClass={onToggleClass}
          onToggleExpand={onToggleExpand}
          onSelectAll={onSelectAll}
          onDeselectAll={onDeselectAll}
          onSearchChange={onSearchChange}
          onParse={onParse}
        />
        <DetailPanel
          apis={apis}
          expandedApis={expandedApis}
          onToggleApi={onToggleApi}
          exportFormats={exportFormats}
          outputDir={outputDir}
          loading={loading}
          onFormatChange={onFormatChange}
          onOutputDirChange={onOutputDirChange}
          onOpenOutputDir={onOpenOutputDir}
          onExport={onExport}
        />
      </div>
    </div>
  )
}
