import { useState, useEffect, useCallback } from 'react'
import './App.css'

declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>
      open_folder_dialog: (title?: string) => Promise<string | null>
      open_file_dialog: (title?: string, filters?: { name: string; extensions: string[] }[]) => Promise<string | null>
    }
  }
}

// --- Types ---

interface ApiDocConfig {
  source_jar_path: string
  service_name: string
  dependency_jars: string[]
  auto_scan_dependencies: boolean
}

interface MethodInfo {
  method_name: string
  http_method: string
  path: string
  api_name: string
}

interface ControllerInfo {
  class_name: string
  class_path: string
  methods: MethodInfo[]
}

interface ApiField {
  field_name: string
  field_type: string
  required: string
  field_length: string
  comment: string
  example_value: string
}

interface NodeInfo {
  node_name: string
  node_desc: string
  resp_fields: ApiField[]
}

interface ApiInfo {
  api_name: string
  http_method: string
  service_name: string
  business_module: string
  method_name: string
  version: string
  full_path: string
  req_fields: ApiField[]
  req_example: string
  resp_nodes: NodeInfo[]
  resp_example: string
}

interface ExportHistory {
  id: string
  service_name: string
  api_count: number
  formats: string[]
  output_path: string
  exported_at: string
}

type ViewMode = 'config' | 'select' | 'preview'

// --- Helpers ---

const defaultConfig: ApiDocConfig = {
  source_jar_path: '',
  service_name: '',
  dependency_jars: [],
  auto_scan_dependencies: false,
}

function App() {
  const [view, setView] = useState<ViewMode>('config')
  const [config, setConfig] = useState<ApiDocConfig>(defaultConfig)
  const [controllers, setControllers] = useState<ControllerInfo[]>([])
  const [selectedMethods, setSelectedMethods] = useState<Set<string>>(new Set())
  const [apis, setApis] = useState<ApiInfo[]>([])
  const [exportFormats, setExportFormats] = useState<string[]>(['markdown'])
  const [outputDir, setOutputDir] = useState('')
  const [outputFiles, setOutputFiles] = useState<string[]>([])
  const [history, setHistory] = useState<ExportHistory[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [toast, setToast] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [expandedClasses, setExpandedClasses] = useState<Set<string>>(new Set())
  const [searchFilter, setSearchFilter] = useState('')
  const [expandedApis, setExpandedApis] = useState<Set<string>>(new Set())
  const [apiReady, setApiReady] = useState(false)

  const showToast = useCallback((type: 'success' | 'error', message: string) => {
    setToast({ type, message })
    setTimeout(() => setToast(null), 3000)
  }, [])

  const callAPI = useCallback(async <T,>(method: string, params?: Record<string, unknown>): Promise<T> => {
    try {
      setError(null)
      return await window.pluginAPI.call('api-doc', method, params ?? {}) as T
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      setError(message)
      showToast('error', message)
      throw e
    }
  }, [showToast])

  // Wait for pluginAPI
  useEffect(() => {
    const waitForAPI = setInterval(() => {
      if (window.pluginAPI) {
        clearInterval(waitForAPI)
        setApiReady(true)
      }
    }, 100)
    return () => clearInterval(waitForAPI)
  }, [])

  // Load config and history when API is ready
  useEffect(() => {
    if (!apiReady) return
    const init = async () => {
      try {
        const savedConfig = await callAPI<ApiDocConfig | null>('load_config')
        if (savedConfig) setConfig({ ...defaultConfig, ...savedConfig })
        const h = await callAPI<ExportHistory[]>('get_export_history')
        setHistory(h)
      } catch {
        // First run, no saved data
      }
    }
    init()
  }, [apiReady, callAPI])

  const handleOpenJar = async () => {
    const path = await window.pluginAPI.open_file_dialog('选择 Spring Boot JAR 文件', [
      { name: 'JAR 文件', extensions: ['jar'] },
    ])
    if (path) {
      setConfig(prev => ({ ...prev, source_jar_path: path }))
    }
  }

  const handleOpenOutputDir = async () => {
    const path = await window.pluginAPI.open_folder_dialog('选择输出目录')
    if (path) setOutputDir(path)
  }

  const handleScan = async () => {
    if (!config.source_jar_path) {
      showToast('error', '请先选择 JAR 文件')
      return
    }
    setLoading(true)
    try {
      await callAPI('save_config', { ...config })
      const result = await callAPI<ControllerInfo[]>('scan_controllers', {
        source_jar_path: config.source_jar_path,
      })
      setControllers(result)
      setSelectedMethods(new Set())
      // Auto expand all
      setExpandedClasses(new Set(result.map(c => c.class_name)))
      setView('select')
      showToast('success', `找到 ${result.length} 个 Controller`)
    } catch {
      // error already handled
    } finally {
      setLoading(false)
    }
  }

  const toggleMethod = (key: string) => {
    setSelectedMethods(prev => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  const toggleClass = (className: string) => {
    const ctrl = controllers.find(c => c.class_name === className)
    if (!ctrl) return
    const methodKeys = ctrl.methods.map(m => `${className}::${m.method_name}`)
    const allSelected = methodKeys.every(k => selectedMethods.has(k))

    setSelectedMethods(prev => {
      const next = new Set(prev)
      methodKeys.forEach(k => {
        if (allSelected) next.delete(k)
        else next.add(k)
      })
      return next
    })
  }

  const selectAll = () => {
    const all = new Set<string>()
    controllers.forEach(c => c.methods.forEach(m => all.add(`${c.class_name}::${m.method_name}`)))
    setSelectedMethods(all)
  }

  const deselectAll = () => setSelectedMethods(new Set())

  const toggleExpandClass = (cn: string) => {
    setExpandedClasses(prev => {
      const next = new Set(prev)
      if (next.has(cn)) next.delete(cn)
      else next.add(cn)
      return next
    })
  }

  const handleParseDetails = async () => {
    if (selectedMethods.size === 0) {
      showToast('error', '请至少选择一个 API')
      return
    }

    setLoading(true)
    try {
      const selected = Array.from(selectedMethods).map(k => {
        const [className, methodName] = k.split('::')
        return [className, methodName]
      })

      const result = await callAPI<ApiInfo[]>('parse_api_details', {
        source_jar_path: config.source_jar_path,
        service_name: config.service_name,
        controllers,
        selected,
        dependency_jars: config.dependency_jars,
        auto_scan_dependencies: config.auto_scan_dependencies,
      })

      setApis(result)
      setExpandedApis(new Set(result.map(a => a.full_path)))
      showToast('success', `解析了 ${result.length} 个 API`)
    } catch {
      // error handled
    } finally {
      setLoading(false)
    }
  }

  const handleExport = async () => {
    if (apis.length === 0) {
      showToast('error', '没有可导出的 API')
      return
    }
    if (exportFormats.length === 0) {
      showToast('error', '请选择至少一种导出格式')
      return
    }
    if (!outputDir) {
      showToast('error', '请选择输出目录')
      return
    }

    setLoading(true)
    try {
      const files = await callAPI<string[]>('export_docs', {
        selected_apis: [],
        output_dir: outputDir,
        formats: exportFormats,
        apis,
        service_name: config.service_name || 'unknown',
      })
      setOutputFiles(files)
      setView('preview')
      showToast('success', `导出了 ${files.length} 个文件`)

      // Refresh history
      const h = await callAPI<ExportHistory[]>('get_export_history')
      setHistory(h)
    } catch {
      // error handled
    } finally {
      setLoading(false)
    }
  }

  const httpMethodColor = (method: string) => {
    switch (method) {
      case 'GET': return 'method-get'
      case 'POST': return 'method-post'
      case 'PUT': return 'method-put'
      case 'DELETE': return 'method-delete'
      case 'PATCH': return 'method-patch'
      default: return ''
    }
  }

  const filteredControllers = controllers.filter(c => {
    if (!searchFilter) return true
    const q = searchFilter.toLowerCase()
    return (
      c.class_name.toLowerCase().includes(q) ||
      c.class_path.toLowerCase().includes(q) ||
      c.methods.some(m => m.path.toLowerCase().includes(q) || m.api_name.toLowerCase().includes(q))
    )
  })

  // --- Render ---

  return (
    <div className="app">
      {toast && <div className={`toast toast-${toast.type}`}>{toast.message}</div>}

      {/* Header with steps */}
      <header className="header">
        <h1>API 文档生成器</h1>
        <div className="steps">
          <div className={`step ${view === 'config' ? 'active' : 'done'}`}>
            <span className="step-num">1</span> 配置
          </div>
          <div className="step-line" />
          <div className={`step ${view === 'select' ? 'active' : ['preview'].includes(view) ? 'done' : ''}`}>
            <span className="step-num">2</span> 选择 & 解析
          </div>
          <div className="step-line" />
          <div className={`step ${view === 'preview' ? 'active' : ''}`}>
            <span className="step-num">3</span> 导出
          </div>
        </div>
      </header>

      {error && <div className="error-banner">{error}</div>}

      {/* Step 1: Config */}
      {view === 'config' && (
        <div className="card">
          <h2>JAR 文件配置</h2>
          <div className="form-group">
            <label>Spring Boot JAR 路径</label>
            <div className="input-row">
              <input
                type="text"
                value={config.source_jar_path}
                onChange={e => setConfig(prev => ({ ...prev, source_jar_path: e.target.value }))}
                placeholder="选择或输入 JAR 文件路径"
              />
              <button onClick={handleOpenJar} className="btn-secondary">浏览</button>
            </div>
          </div>
          <div className="form-group">
            <label>服务名称</label>
            <input
              type="text"
              value={config.service_name}
              onChange={e => setConfig(prev => ({ ...prev, service_name: e.target.value }))}
              placeholder="如: user-service"
            />
          </div>
          <div className="form-group">
            <label>
              <input
                type="checkbox"
                checked={config.auto_scan_dependencies}
                onChange={e => setConfig(prev => ({ ...prev, auto_scan_dependencies: e.target.checked }))}
              />
              自动扫描依赖 JAR
            </label>
          </div>
          <div className="form-group">
            <label>依赖 JAR 前缀 (逗号分隔)</label>
            <input
              type="text"
              value={config.dependency_jars.join(',')}
              onChange={e => setConfig(prev => ({
                ...prev,
                dependency_jars: e.target.value.split(',').map(s => s.trim()).filter(Boolean),
              }))}
              placeholder="如: my-company,shared-lib"
              disabled={config.auto_scan_dependencies}
            />
          </div>

          <button onClick={handleScan} className="btn-primary" disabled={loading}>
            {loading ? '扫描中...' : '扫描 Controller'}
          </button>

          {history.length > 0 && (
            <div className="history-section">
              <h3>导出历史</h3>
              <div className="history-list">
                {history.slice(-5).reverse().map(h => (
                  <div key={h.id} className="history-item">
                    <span className="history-name">{h.service_name}</span>
                    <span className="history-count">{h.api_count} APIs</span>
                    <span className="history-time">{new Date(h.exported_at).toLocaleString()}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Step 2: Select */}
      {view === 'select' && (
        <div className="select-view">
          <div className="select-header">
            <button onClick={() => setView('config')} className="btn-secondary">← 返回</button>
            <input
              type="text"
              className="search-input"
              placeholder="搜索 Controller 或 API..."
              value={searchFilter}
              onChange={e => setSearchFilter(e.target.value)}
            />
            <div className="select-actions">
              <button onClick={selectAll} className="btn-small">全选</button>
              <button onClick={deselectAll} className="btn-small">取消全选</button>
              <span className="select-count">已选 {selectedMethods.size} 个 API</span>
            </div>
          </div>

          <div className="controller-tree">
            {filteredControllers.map(ctrl => {
              const allMethodKeys = ctrl.methods.map(m => `${ctrl.class_name}::${m.method_name}`)
              const allSelected = allMethodKeys.length > 0 && allMethodKeys.every(k => selectedMethods.has(k))
              const someSelected = allMethodKeys.some(k => selectedMethods.has(k))
              const isExpanded = expandedClasses.has(ctrl.class_name)

              return (
                <div key={ctrl.class_name} className="controller-group">
                  <div className="controller-header">
                    <input
                      type="checkbox"
                      checked={allSelected}
                      ref={el => { if (el) el.indeterminate = !allSelected && someSelected }}
                      onChange={() => toggleClass(ctrl.class_name)}
                    />
                    <span className="expand-btn" onClick={() => toggleExpandClass(ctrl.class_name)}>
                      {isExpanded ? '▼' : '▶'}
                    </span>
                    <span className="controller-name">{ctrl.class_name}</span>
                    <span className="controller-path">{ctrl.class_path}</span>
                    <span className="method-count">{ctrl.methods.length} 个接口</span>
                  </div>

                  {isExpanded && (
                    <div className="method-list">
                      {ctrl.methods.map(method => {
                        const key = `${ctrl.class_name}::${method.method_name}`
                        return (
                          <div key={key} className="method-item">
                            <input
                              type="checkbox"
                              checked={selectedMethods.has(key)}
                              onChange={() => toggleMethod(key)}
                            />
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
            <div className="parse-section">
              <button onClick={handleParseDetails} className="btn-primary" disabled={loading}>
                {loading ? '解析中...' : `解析 ${selectedMethods.size} 个 API`}
              </button>
            </div>
          )}

          {apis.length > 0 && (
            <div className="api-results">
              <h2>解析结果 ({apis.length} 个 API)</h2>
              {apis.map(api => {
                const isExpanded = expandedApis.has(api.full_path)
                return (
                  <div key={api.full_path} className="api-card">
                    <div className="api-card-header" onClick={() => {
                      setExpandedApis(prev => {
                        const next = new Set(prev)
                        if (next.has(api.full_path)) next.delete(api.full_path)
                        else next.add(api.full_path)
                        return next
                      })
                    }}>
                      <span className={`method-badge ${httpMethodColor(api.http_method)}`}>
                        {api.http_method}
                      </span>
                      <span className="api-card-path">{api.full_path}</span>
                      <span className="api-card-name">{api.api_name}</span>
                      <span className="expand-btn">{isExpanded ? '▼' : '▶'}</span>
                    </div>

                    {isExpanded && (
                      <div className="api-card-body">
                        {api.req_fields.length > 0 && (
                          <>
                            <h4>请求参数</h4>
                            <table>
                              <thead>
                                <tr><th>字段名</th><th>类型</th><th>必填</th><th>注释</th></tr>
                              </thead>
                              <tbody>
                                {api.req_fields.map(f => (
                                  <tr key={f.field_name}>
                                    <td><code>{f.field_name}</code></td>
                                    <td>{f.field_type}</td>
                                    <td>{f.required}</td>
                                    <td>{f.comment}</td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </>
                        )}
                        {api.req_example && (
                          <div className="code-block">
                            <h4>请求示例</h4>
                            <pre><code>{api.req_example}</code></pre>
                          </div>
                        )}
                        {api.resp_nodes.length > 0 && (
                          <>
                            <h4>响应参数</h4>
                            {api.resp_nodes.map(node => (
                              <div key={node.node_name} className="resp-node">
                                <h5>{node.node_name} {node.node_desc && `(${node.node_desc})`}</h5>
                                <table>
                                  <thead><tr><th>字段名</th><th>类型</th><th>注释</th></tr></thead>
                                  <tbody>
                                    {node.resp_fields.map(f => (
                                      <tr key={f.field_name}>
                                        <td><code>{f.field_name}</code></td>
                                        <td>{f.field_type}</td>
                                        <td>{f.comment}</td>
                                      </tr>
                                    ))}
                                  </tbody>
                                </table>
                              </div>
                            ))}
                          </>
                        )}
                        {api.resp_example && (
                          <div className="code-block">
                            <h4>响应示例</h4>
                            <pre><code>{api.resp_example}</code></pre>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                )
              })}

              <div className="export-section">
                <h3>导出设置</h3>
                <div className="form-group">
                  <label>导出格式</label>
                  <div className="format-checkboxes">
                    {['markdown', 'word', 'html'].map(fmt => (
                      <label key={fmt} className="format-option">
                        <input
                          type="checkbox"
                          checked={exportFormats.includes(fmt)}
                          onChange={e => {
                            if (e.target.checked) setExportFormats(prev => [...prev, fmt])
                            else setExportFormats(prev => prev.filter(f => f !== fmt))
                          }}
                        />
                        {fmt.toUpperCase()}
                      </label>
                    ))}
                  </div>
                </div>
                <div className="form-group">
                  <label>输出目录</label>
                  <div className="input-row">
                    <input type="text" value={outputDir} onChange={e => setOutputDir(e.target.value)} placeholder="选择输出目录" />
                    <button onClick={handleOpenOutputDir} className="btn-secondary">浏览</button>
                  </div>
                </div>
                <button onClick={handleExport} className="btn-primary" disabled={loading}>
                  {loading ? '导出中...' : `导出 ${apis.length} 个 API`}
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Step 3: Preview */}
      {view === 'preview' && (
        <div className="card">
          <h2>导出完成</h2>
          <div className="export-results">
            {outputFiles.map((f, i) => (
              <div key={i} className="export-result-item">
                <span className="result-icon">✓</span>
                <span className="result-file">{f}</span>
              </div>
            ))}
          </div>
          <div className="export-actions">
            <button onClick={() => { setView('select'); setOutputFiles([]) }} className="btn-secondary">
              返回
            </button>
            <button onClick={() => { setView('config'); setApis([]); setControllers([]); setSelectedMethods(new Set()) }} className="btn-secondary">
              重新开始
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
