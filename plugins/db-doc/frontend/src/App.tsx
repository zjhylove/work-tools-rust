import { useState, useEffect, useMemo } from 'react'
import './App.css'

// 类型定义
interface ToastMessage {
  id: number
  type: 'success' | 'error' | 'info'
  message: string
}

interface ConnectionConfig {
  id: string
  name: string
  db_type: 'mysql' | 'postgresql'
  host: string
  port: number
  database: string
  username: string
  password?: string
  created_at: number
  last_used?: number
}

interface TableInfo {
  name: string
  schema: string
  comment?: string
  columns: ColumnInfo[]
  indexes: IndexInfo[]
}

interface ColumnInfo {
  name: string
  data_type: string
  max_length?: number
  is_nullable: boolean
  is_primary_key: boolean
  default_value?: string
  comment?: string
  position: number
}

interface IndexInfo {
  name: string
  columns: string[]
  is_unique: boolean
  is_primary: boolean
}

declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>
      open_folder_dialog: (title?: string) => Promise<string | null>
    }
  }
}

type ViewMode = 'connections' | 'select'

function App() {
  const [connections, setConnections] = useState<ConnectionConfig[]>([])
  const [selectedConnection, setSelectedConnection] = useState<ConnectionConfig | null>(null)
  const [tables, setTables] = useState<string[]>([])
  const [selectedTables, setSelectedTables] = useState<Set<string>>(new Set())
  const [tableInfos, setTableInfos] = useState<TableInfo[]>([])
  const [viewMode, setViewMode] = useState<ViewMode>('connections')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [toasts, setToasts] = useState<ToastMessage[]>([])
  const [connectionSearch, setConnectionSearch] = useState('')
  const [testingConnection, setTestingConnection] = useState(false)
  const [testResult, setTestResult] = useState<{ id: string; success: boolean; message?: string } | null>(null)
  const [tableSearch, setTableSearch] = useState('')
  const [prefixFilter, setPrefixFilter] = useState('')
  // 导出状态
  const [exportFormats, setExportFormats] = useState<Set<'markdown' | 'html'>>(new Set(['markdown', 'html']))
  const [outputDir, setOutputDir] = useState('')
  const [exporting, setExporting] = useState(false)

  const filteredConnections = useMemo(
    () => connections.filter(c => c.name.toLowerCase().includes(connectionSearch.toLowerCase())),
    [connections, connectionSearch]
  )

  const filteredTables = useMemo(
    () => tables.filter(t => {
      const matchesSearch = t.toLowerCase().includes(tableSearch.toLowerCase())
      const matchesPrefix = prefixFilter === '' || t.toLowerCase().startsWith(prefixFilter.toLowerCase())
      return matchesSearch && matchesPrefix
    }),
    [tables, tableSearch, prefixFilter]
  )

  const selectByPrefix = () => {
    const matching = tables.filter(t => t.toLowerCase().startsWith(prefixFilter.toLowerCase()))
    setSelectedTables(new Set([...selectedTables, ...matching]))
  }

  const invertSelection = () => {
    const newSelected = new Set<string>()
    tables.forEach(t => {
      if (!selectedTables.has(t)) newSelected.add(t)
    })
    setSelectedTables(newSelected)
  }

  const showToast = (type: ToastMessage['type'], message: string) => {
    const id = Date.now()
    setToasts(prev => [...prev, { id, type, message }])
    setTimeout(() => setToasts(prev => prev.filter(t => t.id !== id)), 3000)
  }

  const callAPI = async <T,>(method: string, params?: Record<string, unknown>): Promise<T> => {
    try {
      setError(null)
      return await window.pluginAPI.call('db-doc', method, params ?? {}) as T
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      setError(message)
      showToast('error', message)
      throw e
    }
  }

  // 加载连接列表
  useEffect(() => {
    const waitForAPI = setInterval(() => {
      if (window.pluginAPI) {
        clearInterval(waitForAPI)
        loadConnections()
      }
    }, 100)
    return () => clearInterval(waitForAPI)
  }, [])

  const loadConnections = async () => {
    try {
      setLoading(true)
      const result = await callAPI<ConnectionConfig[]>('list_connections', {})
      setConnections(result || [])
    } catch { /* callAPI handles error */ } finally {
      setLoading(false)
    }
  }

  // 测试连接
  const testConnection = async (config: Partial<ConnectionConfig>) => {
    try {
      setTestingConnection(true)
      setTestResult(null)
      const result = await callAPI<{ success: boolean; message: string }>('test_connection', config as Record<string, unknown>)
      setTestResult({ id: config.id || '', success: result.success, message: result.message })
      if (result.success) {
        showToast('success', '连接成功')
      } else {
        showToast('error', '连接失败: ' + result.message)
      }
    } catch (e) {
      setTestResult({ id: config.id || '', success: false, message: e instanceof Error ? e.message : '未知错误' })
    } finally {
      setTestingConnection(false)
    }
  }

  // 选择连接进入表选择视图
  const handleSelectConnection = async (conn: ConnectionConfig) => {
    setSelectedConnection(conn)
    setTableInfos([])
    setSelectedTables(new Set())
    setOutputDir('')
    try {
      setLoading(true)
      const result = await callAPI<string[]>('list_tables', { connection_id: conn.id })
      setTables(result || [])
      setViewMode('select')
    } catch { /* callAPI handles error */ } finally {
      setLoading(false)
    }
  }

  // 删除连接
  const handleDeleteConnection = async (connId: string) => {
    try {
      await callAPI('delete_connection', { id: connId })
      showToast('success', '连接已删除')
      loadConnections()
      if (selectedConnection?.id === connId) {
        setSelectedConnection(null)
        setViewMode('connections')
      }
    } catch { /* callAPI handles error */ }
  }

  // 获取表详情
  const loadTableInfo = async (connectionId: string, tableName: string): Promise<TableInfo | null> => {
    try {
      return await callAPI<TableInfo>('get_table_info', {
        connection_id: connectionId,
        table_name: tableName
      })
    } catch { return null }
  }

  // 切换表选择
  const toggleTableSelection = (tableName: string) => {
    const newSelected = new Set(selectedTables)
    if (newSelected.has(tableName)) {
      newSelected.delete(tableName)
    } else {
      newSelected.add(tableName)
    }
    setSelectedTables(newSelected)
  }

  // 全选/取消全选
  const toggleSelectAll = () => {
    if (selectedTables.size === tables.length) {
      setSelectedTables(new Set())
    } else {
      setSelectedTables(new Set(tables))
    }
  }

  // 加载选中表的预览
  const handleLoadPreview = async () => {
    if (!selectedConnection || selectedTables.size === 0) return
    setLoading(true)
    try {
      const infos = await Promise.all(
        Array.from(selectedTables).map(tableName =>
          loadTableInfo(selectedConnection.id, tableName)
        )
      )
      setTableInfos(infos.filter((info): info is TableInfo => info !== null))
    } finally {
      setLoading(false)
    }
  }

  // 选择输出目录
  const handlePickDir = async () => {
    const dir = await window.pluginAPI.open_folder_dialog('选择导出目录')
    if (dir) setOutputDir(dir)
  }

  // 切换导出格式
  const toggleExportFormat = (fmt: 'markdown' | 'html') => {
    const next = new Set(exportFormats)
    if (next.has(fmt)) {
      if (next.size > 1) next.delete(fmt)
    } else {
      next.add(fmt)
    }
    setExportFormats(next)
  }

  // 导出文档
  const handleExport = async () => {
    if (!selectedConnection || selectedTables.size === 0 || !outputDir || exportFormats.size === 0) return
    setExporting(true)
    try {
      for (const fmt of exportFormats) {
        const result = await callAPI<{ success: boolean; files?: string[]; message?: string }>('export_docs', {
          connection_id: selectedConnection.id,
          connection_name: selectedConnection.name,
          tables: Array.from(selectedTables),
          output_dir: outputDir,
          format: fmt
        })
        if (result.success) {
          showToast('success', `${fmt.toUpperCase()} 导出成功`)
        } else {
          showToast('error', `${fmt.toUpperCase()} 导出失败: ` + (result.message || '未知错误'))
        }
      }
    } catch (e) {
      showToast('error', '导出失败: ' + (e instanceof Error ? e.message : '未知错误'))
    } finally {
      setExporting(false)
    }
  }

  const step = viewMode === 'connections' ? 1 : 2

  return (
    <div className="app">
      <StepHeader step={step} onStepClick={(s) => {
        if (s === 1) setViewMode('connections')
      }} />

      {error && <div className="error-banner">{error}</div>}

      {viewMode === 'connections' && (
        <ConnectionView
          connections={filteredConnections}
          allConnections={connections}
          search={connectionSearch}
          onSearchChange={setConnectionSearch}
          onSelect={handleSelectConnection}
          onDelete={handleDeleteConnection}
          onTest={testConnection}
          testing={testingConnection}
          testResult={testResult}
          onSave={async (config) => {
            await callAPI('save_connection', config as Record<string, unknown>)
            loadConnections()
          }}
          onUpdate={async (config) => {
            await callAPI('update_connection', config as Record<string, unknown>)
            loadConnections()
          }}
        />
      )}

      {viewMode === 'select' && selectedConnection && (
        <div className="view-container view-container--split">
          <div className="panel panel--left">
            <div className="panel-header">
              <button className="btn-back" onClick={() => setViewMode('connections')}>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M10 12L6 8L10 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
                返回
              </button>
              <h2>{selectedConnection.name}</h2>
            </div>

            <div className="table-search-bar">
              <input
                className="search-input"
                type="text"
                placeholder="搜索表名..."
                value={tableSearch}
                onChange={(e) => setTableSearch(e.target.value)}
              />
            </div>

            <div className="table-toolbar">
              <input
                className="prefix-input"
                type="text"
                placeholder="表前缀"
                value={prefixFilter}
                onChange={(e) => setPrefixFilter(e.target.value)}
              />
              <button onClick={selectByPrefix} disabled={!prefixFilter}>按前缀</button>
              <button onClick={invertSelection}>反选</button>
            </div>

            <div className="table-actions-top">
              <button onClick={toggleSelectAll}>
                {selectedTables.size === tables.length ? '取消全选' : '全选'}
              </button>
              <span className="selected-count">{selectedTables.size}/{tables.length}</span>
            </div>

            <div className="table-list">
              {filteredTables.map((table) => (
                <label
                  key={table}
                  className={`table-item ${selectedTables.has(table) ? 'table-item--selected' : ''}`}
                >
                  <input
                    type="checkbox"
                    checked={selectedTables.has(table)}
                    onChange={() => toggleTableSelection(table)}
                  />
                  <span className="table-item-name">{table}</span>
                </label>
              ))}
              {filteredTables.length === 0 && (
                <p className="empty-hint">{tables.length === 0 ? '暂无表' : '无匹配表'}</p>
              )}
            </div>

            <div className="panel-footer">
              <button
                className="btn-primary"
                disabled={selectedTables.size === 0}
                onClick={handleLoadPreview}
              >
                加载预览 ({selectedTables.size})
              </button>
            </div>
          </div>

          <div className="panel panel--right">
            {loading ? (
              <div className="loading-state">
                <div className="spinner" />
                <span>加载中...</span>
              </div>
            ) : tableInfos.length === 0 ? (
              <div className="empty-state">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                  <rect x="3" y="3" width="18" height="18" rx="2" />
                  <path d="M3 9h18M9 3v18" />
                </svg>
                <p>选择表后点击"加载预览"查看结构</p>
              </div>
            ) : (
              <div className="preview-content">
                {tableInfos.map((table) => (
                  <TablePreview key={table.name} table={table} />
                ))}
              </div>
            )}

            {tableInfos.length > 0 && (
              <div className="export-panel">
                <div className="export-panel-inner">
                  <div className="export-formats">
                    <span className="export-label">导出格式</span>
                    <label className={`format-chip ${exportFormats.has('markdown') ? 'format-chip--active' : ''}`}>
                      <input
                        type="checkbox"
                        checked={exportFormats.has('markdown')}
                        onChange={() => toggleExportFormat('markdown')}
                      />
                      Markdown
                    </label>
                    <label className={`format-chip ${exportFormats.has('html') ? 'format-chip--active' : ''}`}>
                      <input
                        type="checkbox"
                        checked={exportFormats.has('html')}
                        onChange={() => toggleExportFormat('html')}
                      />
                      HTML
                    </label>
                  </div>

                  <div className="export-dir">
                    <span className="export-label">输出目录</span>
                    <div className="export-dir-row">
                      <input
                        className="dir-input"
                        type="text"
                        value={outputDir}
                        onChange={(e) => setOutputDir(e.target.value)}
                        placeholder="选择导出目录..."
                        readOnly
                      />
                      <button onClick={handlePickDir}>浏览</button>
                    </div>
                  </div>

                  <button
                    className="btn-primary export-btn"
                    disabled={exporting || !outputDir || exportFormats.size === 0 || selectedTables.size === 0}
                    onClick={handleExport}
                  >
                    {exporting ? '导出中...' : `导出 ${selectedTables.size} 张表`}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {toasts.length > 0 && (
        <div className="toast-container">
          {toasts.map(toast => (
            <div key={toast.id} className={`toast toast--${toast.type}`}>
              {toast.type === 'success' && '✓ '}
              {toast.type === 'error' && '✗ '}
              {toast.message}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// StepHeader 组件
function StepHeader({ step, onStepClick }: { step: number; onStepClick: (s: number) => void }) {
  const steps = [
    { num: 1, label: '连接管理' },
    { num: 2, label: '选择表 & 导出' },
  ]

  return (
    <div className="step-header">
      {steps.map((s, i) => (
        <div key={s.num} className="step-group">
          <div
            className={`step-node ${step === s.num ? 'step-node--active' : step > s.num ? 'step-node--done' : ''}`}
            onClick={() => onStepClick(s.num)}
          >
            <span className="step-node-num">{step > s.num ? '✓' : s.num}</span>
            <span className="step-node-label">{s.label}</span>
          </div>
          {i < steps.length - 1 && <div className="step-connector" />}
        </div>
      ))}
    </div>
  )
}

// 连接管理视图
function ConnectionView({
  connections, allConnections, search, onSearchChange, onSelect, onDelete, onTest,
  testing, testResult, onSave, onUpdate
}: {
  connections: ConnectionConfig[]
  allConnections: ConnectionConfig[]
  search: string
  onSearchChange: (v: string) => void
  onSelect: (c: ConnectionConfig) => void
  onDelete: (id: string) => void
  onTest: (c: Partial<ConnectionConfig>) => Promise<void>
  testing: boolean
  testResult: { id: string; success: boolean; message?: string } | null
  onSave: (c: Partial<ConnectionConfig>) => Promise<void>
  onUpdate: (c: Partial<ConnectionConfig>) => Promise<void>
}) {
  const [editingId, setEditingId] = useState<string | null>(null)

  return (
    <div className="connection-layout">
      <div className="connection-list-panel">
        <h2>已保存的连接</h2>
        {allConnections.length > 0 && (
          <input
            className="search-input"
            type="text"
            placeholder="搜索连接..."
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
          />
        )}
        {connections.length === 0 ? (
          <p className="empty-hint">{allConnections.length === 0 ? '暂无保存的连接' : '无匹配连接'}</p>
        ) : (
          <ul className="connection-items">
            {connections.map((conn) => (
              <li key={conn.id} className="connection-card">
                <div className="connection-card-main" onClick={() => onSelect(conn)}>
                  <div className="connection-card-top">
                    <span className="connection-card-name">{conn.name}</span>
                    <span className={`db-type-badge db-type-badge--${conn.db_type}`}>{conn.db_type.toUpperCase()}</span>
                  </div>
                  <div className="connection-card-meta">
                    <span>{conn.host}:{conn.port}</span>
                    <span className="meta-sep">/</span>
                    <span>{conn.database}</span>
                  </div>
                  {testResult && testResult.id === conn.id && (
                    <span className={`test-indicator ${testResult.success ? 'test-indicator--ok' : 'test-indicator--fail'}`}>
                      {testResult.success ? '✓' : '✗'}
                    </span>
                  )}
                </div>
                <div className="connection-card-actions">
                  <button onClick={() => { setEditingId(editingId === conn.id ? null : conn.id); onSelect(conn) }}>选择</button>
                  <button onClick={() => onTest(conn)} disabled={testing}>
                    {testing && testResult?.id === conn.id ? '测试中...' : '测试'}
                  </button>
                  <button className="btn-danger" onClick={() => onDelete(conn.id)}>删除</button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="connection-form-panel">
        <h2>{editingId ? '编辑连接' : '新建连接'}</h2>
        <ConnectionForm
          key={editingId || 'new'}
          initial={editingId ? connections.find(c => c.id === editingId) : undefined}
          onSave={async (config) => {
            if (editingId) {
              await onUpdate({ ...config, id: editingId })
            } else {
              await onSave(config)
            }
            setEditingId(null)
          }}
          onTest={onTest}
        />
      </div>
    </div>
  )
}

// 连接表单
function ConnectionForm({
  initial, onSave, onTest
}: {
  initial?: Partial<ConnectionConfig>
  onSave: (c: Partial<ConnectionConfig>) => Promise<void>
  onTest: (c: Partial<ConnectionConfig>) => Promise<void>
}) {
  const [config, setConfig] = useState<Partial<ConnectionConfig>>({
    name: '',
    db_type: 'mysql',
    host: 'localhost',
    port: 3306,
    database: '',
    username: 'root',
    password: '',
    ...initial,
  })
  const [saving, setSaving] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    try { await onSave(config) } finally { setSaving(false) }
  }

  return (
    <form onSubmit={handleSubmit} className="form">
      <div className="form-group">
        <label className="form-label">连接名称</label>
        <input
          className="form-input"
          value={config.name}
          onChange={(e) => setConfig({ ...config, name: e.target.value })}
          placeholder="例如: 生产环境"
          required
        />
      </div>

      <div className="form-group">
        <label className="form-label">数据库类型</label>
        <select
          className="form-input"
          value={config.db_type}
          onChange={(e) => setConfig({
            ...config,
            db_type: e.target.value as 'mysql' | 'postgresql',
            port: e.target.value === 'mysql' ? 3306 : 5432
          })}
        >
          <option value="mysql">MySQL</option>
          <option value="postgresql">PostgreSQL</option>
        </select>
      </div>

      <div className="form-row">
        <div className="form-group">
          <label className="form-label">主机</label>
          <input
            className="form-input"
            value={config.host}
            onChange={(e) => setConfig({ ...config, host: e.target.value })}
            placeholder="localhost"
          />
        </div>
        <div className="form-group">
          <label className="form-label">端口</label>
          <input
            className="form-input"
            type="number"
            value={config.port}
            onChange={(e) => setConfig({ ...config, port: parseInt(e.target.value) })}
          />
        </div>
      </div>

      <div className="form-group">
        <label className="form-label">数据库名</label>
        <input
          className="form-input"
          value={config.database}
          onChange={(e) => setConfig({ ...config, database: e.target.value })}
          placeholder="database_name"
          required
        />
      </div>

      <div className="form-group">
        <label className="form-label">用户名</label>
        <input
          className="form-input"
          value={config.username}
          onChange={(e) => setConfig({ ...config, username: e.target.value })}
          placeholder="root"
        />
      </div>

      <div className="form-group">
        <label className="form-label">密码</label>
        <input
          className="form-input"
          type="password"
          value={config.password}
          onChange={(e) => setConfig({ ...config, password: e.target.value })}
          placeholder="••••••••"
        />
      </div>

      <div className="form-actions">
        <button type="button" className="btn-secondary" onClick={() => onTest(config)}>测试连接</button>
        <button type="submit" className="btn-primary" disabled={saving}>
          {saving ? '保存中...' : '保存'}
        </button>
      </div>
    </form>
  )
}

// 表预览组件
function TablePreview({ table }: { table: TableInfo }) {
  return (
    <div className="table-preview">
      <div className="table-preview-header">
        <h3>{table.name}</h3>
        {table.comment && <span className="table-comment-tag">{table.comment}</span>}
      </div>
      <span className="table-schema">库: {table.schema}</span>

      <table className="columns-table">
        <thead>
          <tr>
            <th>字段名</th>
            <th>类型</th>
            <th>可空</th>
            <th>主键</th>
            <th>默认值</th>
            <th>说明</th>
          </tr>
        </thead>
        <tbody>
          {table.columns.map((col) => (
            <tr key={col.name}>
              <td className="col-name">{col.name}</td>
              <td className="col-type">{col.data_type.toUpperCase()}{col.max_length ? `(${col.max_length})` : ''}</td>
              <td>{col.is_nullable ? '是' : '否'}</td>
              <td>{col.is_primary_key ? '是' : ''}</td>
              <td className="col-default">{col.default_value || '-'}</td>
              <td className="col-comment">{col.comment || '-'}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {table.indexes.length > 0 && (
        <div className="indexes-section">
          <h4>索引</h4>
          <table className="indexes-table">
            <thead>
              <tr>
                <th>索引名</th>
                <th>列</th>
                <th>唯一</th>
                <th>类型</th>
              </tr>
            </thead>
            <tbody>
              {table.indexes.map((idx) => (
                <tr key={idx.name}>
                  <td>{idx.name}</td>
                  <td>{idx.columns.join(', ')}</td>
                  <td>{idx.is_unique ? '是' : '否'}</td>
                  <td>{idx.is_primary ? '主键' : '普通'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

export default App
