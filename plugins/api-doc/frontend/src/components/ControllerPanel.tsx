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

// 高亮搜索匹配的文本
function highlightMatch(text: string, query: string): React.ReactElement {
  if (!query) return <span>{text}</span>

  const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi')
  const parts = text.split(regex)

  return (
    <span>
      {parts.map((part, i) =>
        regex.test(part) ? (
          <mark key={i} className="search-highlight">{part}</mark>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </span>
  )
}

// 判断方法是否匹配搜索条件
function methodMatches(method: any, query: string): boolean {
  if (!query) return true
  const q = query.toLowerCase()
  return (
    method.path?.toLowerCase().includes(q) ||
    method.api_name?.toLowerCase().includes(q) ||
    method.method_name?.toLowerCase().includes(q) ||
    method.http_method?.toLowerCase().includes(q)
  )
}

// 判断 Controller 是否匹配搜索条件
function controllerMatches(ctrl: ControllerInfo, query: string): boolean {
  if (!query) return true
  const q = query.toLowerCase()
  return (
    ctrl.class_name.toLowerCase().includes(q) ||
    ctrl.class_path.toLowerCase().includes(q) ||
    ctrl.methods.some(m => methodMatches(m, q))
  )
}

export default function ControllerPanel({
  controllers, selectedMethods, expandedClasses, searchFilter, loading,
  onBack, onToggleMethod, onToggleClass, onToggleExpand, onSelectAll, onDeselectAll,
  onSearchChange, onParse,
}: Props) {
  const query = searchFilter.trim()

  // 过滤 Controller：保留匹配的 Controller
  const filteredControllers = controllers.filter(c => controllerMatches(c, query))

  // 计算每个 Controller 中匹配的方法数量
  const controllersWithMatchInfo = filteredControllers.map(ctrl => {
    const matchingMethods = query
      ? ctrl.methods.filter(m => methodMatches(m, query))
      : ctrl.methods

    const shouldAutoExpand = query && matchingMethods.length > 0 &&
      matchingMethods.length < ctrl.methods.length

    return {
      ctrl,
      matchingMethods,
      shouldAutoExpand,
    }
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
        {controllersWithMatchInfo.map(({ ctrl, matchingMethods, shouldAutoExpand }) => {
          const displayMethods = query ? matchingMethods : ctrl.methods
          const allMethodKeys = displayMethods.map(m => `${ctrl.class_name}::${m.method_name}`)
          const allSelected = allMethodKeys.length > 0 && allMethodKeys.every(k => selectedMethods.has(k))
          const someSelected = allMethodKeys.some(k => selectedMethods.has(k))

          // 搜索时自动展开，或者用户手动展开
          const isExpanded = shouldAutoExpand || expandedClasses.has(ctrl.class_name)

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
                <span className="controller-name">
                  {highlightMatch(ctrl.class_name, query)}
                </span>
                <span className="controller-path">
                  {highlightMatch(ctrl.class_path, query)}
                </span>
                <span className="method-count-badge">
                  {query ? `${matchingMethods.length}/${ctrl.methods.length}` : ctrl.methods.length}
                </span>
              </div>

              {isExpanded && (
                <div className="method-list">
                  {displayMethods.map(method => {
                    const key = `${ctrl.class_name}::${method.method_name}`
                    const fullPath = `${ctrl.class_path}${method.path}`

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
                          {highlightMatch(method.http_method, query)}
                        </span>
                        <span className="method-path">
                          {highlightMatch(fullPath, query)}
                        </span>
                        {method.api_name && (
                          <span className="method-api-name">
                            {highlightMatch(method.api_name, query)}
                          </span>
                        )}
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
