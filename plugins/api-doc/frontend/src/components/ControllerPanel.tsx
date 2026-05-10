import { ControllerInfo } from '../types'
import { httpMethodColor } from '../types'

interface Props {
  controllers: ControllerInfo[]
  selectedMethods: Set<string>
  expandedClasses: Set<string>
  searchFilter: string
  loading: boolean
  onBack: () => void
  onToggleMethod: (key: string) => void
  onToggleClass: (className: string) => void
  onToggleExpand: (cn: string) => void
  onSelectAll: () => void
  onDeselectAll: () => void
  onSearchChange: (v: string) => void
  onParse: () => void
}

export default function ControllerPanel({
  controllers, selectedMethods, expandedClasses, searchFilter, loading,
  onBack, onToggleMethod, onToggleClass, onToggleExpand, onSelectAll, onDeselectAll,
  onSearchChange, onParse,
}: Props) {
  const filteredControllers = controllers.filter(c => {
    if (!searchFilter) return true
    const q = searchFilter.toLowerCase()
    return (
      c.class_name.toLowerCase().includes(q) ||
      c.class_path.toLowerCase().includes(q) ||
      c.methods.some(m => m.path.toLowerCase().includes(q) || m.api_name.toLowerCase().includes(q))
    )
  })

  return (
    <div className="panel panel--left">
      <div className="panel-toolbar">
        <button onClick={onBack} className="btn btn--ghost btn--xs" title="返回配置">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
        </button>
        <div className="search-box">
          <svg className="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            type="text"
            className="search-input"
            placeholder="搜索 Controller 或 API..."
            value={searchFilter}
            onChange={e => onSearchChange(e.target.value)}
          />
        </div>
        <div className="toolbar-actions">
          <button onClick={onSelectAll} className="btn btn--ghost btn--xs">全选</button>
          <button onClick={onDeselectAll} className="btn btn--ghost btn--xs">清除</button>
        </div>
      </div>

      <div className="select-summary">
        <span className="select-count-badge">{selectedMethods.size}</span>
        <span className="select-count-text">已选 API</span>
      </div>

      <div className="controller-tree">
        {filteredControllers.map(ctrl => {
          const allMethodKeys = ctrl.methods.map(m => `${ctrl.class_name}::${m.method_name}`)
          const allSelected = allMethodKeys.length > 0 && allMethodKeys.every(k => selectedMethods.has(k))
          const someSelected = allMethodKeys.some(k => selectedMethods.has(k))
          const isExpanded = expandedClasses.has(ctrl.class_name)

          return (
            <div key={ctrl.class_name} className="controller-group">
              <div className="controller-header" onClick={() => onToggleExpand(ctrl.class_name)}>
                <label className="checkbox-wrap" onClick={e => e.stopPropagation()}>
                  <input
                    type="checkbox"
                    checked={allSelected}
                    ref={el => { if (el) el.indeterminate = !allSelected && someSelected }}
                    onChange={() => onToggleClass(ctrl.class_name)}
                  />
                  <span className="checkbox-mark" />
                </label>
                <span className={`expand-arrow ${isExpanded ? 'expanded' : ''}`}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                </span>
                <span className="controller-name">{ctrl.class_name}</span>
                <span className="controller-path">{ctrl.class_path}</span>
                <span className="method-count-badge">{ctrl.methods.length}</span>
              </div>

              {isExpanded && (
                <div className="method-list">
                  {ctrl.methods.map(method => {
                    const key = `${ctrl.class_name}::${method.method_name}`
                    return (
                      <div key={key} className={`method-item ${selectedMethods.has(key) ? 'method-item--selected' : ''}`}>
                        <label className="checkbox-wrap" onClick={e => e.stopPropagation()}>
                          <input
                            type="checkbox"
                            checked={selectedMethods.has(key)}
                            onChange={() => onToggleMethod(key)}
                          />
                          <span className="checkbox-mark" />
                        </label>
                        <span className={`method-badge ${httpMethodColor(method.http_method)}`}>
                          {method.http_method}
                        </span>
                        <span className="method-path">{ctrl.class_path}{method.path}</span>
                        {method.api_name && <span className="method-api-name">{method.api_name}</span>}
                      </div>
                    )
                  })}
                </div>
              )}
            </div>
          )
        })}
      </div>

      {selectedMethods.size > 0 && (
        <div className="panel-footer">
          <button onClick={onParse} className="btn btn--primary btn--block" disabled={loading}>
            {loading ? (
              <><span className="spinner" /> 解析中...</>
            ) : `解析 ${selectedMethods.size} 个 API`}
          </button>
        </div>
      )}
    </div>
  )
}
