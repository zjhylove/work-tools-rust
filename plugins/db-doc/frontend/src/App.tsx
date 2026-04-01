import { useState, useEffect } from 'react'
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

// 声明 window.pluginAPI
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>
      open_folder_dialog: (title?: string) => Promise<string | null>
    }
  }
}

type ViewMode = 'connections' | 'tables' | 'preview'

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
  const [testResult, setTestResult] = useState<{id: string, success: boolean, message?: string} | null>(null)
  const [tableSearch, setTableSearch] = useState('')
  const [prefixFilter, setPrefixFilter] = useState('')
  const [showExportPanel, setShowExportPanel] = useState(false)
  const [exportFormat, setExportFormat] = useState<'markdown' | 'word' | 'pdf'>('markdown')
  const [exportTemplate, setExportTemplate] = useState<'simple' | 'detailed'>('detailed')
  const [exporting, setExporting] = useState(false)

  const filteredConnections = connections.filter(c => c.name.toLowerCase().includes(connectionSearch.toLowerCase()))

  const filteredTables = tables.filter(t => {
    const matchesSearch = t.toLowerCase().includes(tableSearch.toLowerCase())
    const matchesPrefix = prefixFilter === '' || t.toLowerCase().startsWith(prefixFilter.toLowerCase())
    return matchesSearch && matchesPrefix
  })

  const selectByPrefix = () => {
    const matching = tables.filter(t => t.toLowerCase().startsWith(prefixFilter.toLowerCase()))
    setSelectedTables(new Set([...selectedTables, ...matching]))
  }

  const invertSelection = () => {
    const newSelected = new Set<string>()
    tables.forEach(t => {
      if (!selectedTables.has(t)) {
        newSelected.add(t)
      }
    })
    setSelectedTables(newSelected)
  }

  const showToast = (type: ToastMessage['type'], message: string) => {
    const id = Date.now()
    setToasts(prev => [...prev, { id, type, message }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 3000)
  }

  const callAPI = async <T,>(method: string, params?: Record<string, unknown>): Promise<T> => {
    try {
      setError(null)
      return await window.pluginAPI.call('db-doc', method, params) as T
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      setError(message)
      showToast('error', message)
      throw e
    }
  }

  // 加载连接列表
  useEffect(() => {
    // 等待 pluginAPI 注入完成后再加载
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
    } catch {
      // callAPI handles error display
    } finally {
      setLoading(false)
    }
  }

  // 测试连接
  const testConnection = async (config: Partial<ConnectionConfig>) => {
    try {
      setTestingConnection(true)
      setTestResult(null)
      const result = await window.pluginAPI.call('db-doc', 'test_connection', config as Record<string, unknown>) as { success: boolean; message: string }
      setTestResult({ id: config.id || '', success: result.success, message: result.message })
      if (result.success) {
        showToast('success', '连接成功!')
      } else {
        showToast('error', '连接失败: ' + result.message)
      }
    } catch (e) {
      setTestResult({ id: config.id || '', success: false, message: e instanceof Error ? e.message : '未知错误' })
      showToast('error', '连接失败: ' + (e instanceof Error ? e.message : '未知错误'))
    } finally {
      setTestingConnection(false)
    }
  }

  // 加载表列表
  const loadTables = async (connectionId: string) => {
    try {
      setLoading(true)
      const result = await callAPI<string[]>('list_tables', { connection_id: connectionId })
      setTables(result || [])
      setSelectedTables(new Set())
    } catch {
      // callAPI handles error display
    } finally {
      setLoading(false)
    }
  }

  // 获取表详情
  const loadTableInfo = async (connectionId: string, tableName: string) => {
    try {
      const result = await callAPI<TableInfo>('get_table_info', {
        connection_id: connectionId,
        table_name: tableName
      })
      return result
    } catch {
      return null
    }
  }

  // 选择连接
  const handleSelectConnection = async (conn: ConnectionConfig) => {
    setSelectedConnection(conn)
    await loadTables(conn.id)
    setViewMode('tables')
  }

  // 删除连接
  const handleDeleteConnection = async (connId: string) => {
    try {
      await callAPI('delete_connection', { id: connId })
      showToast('success', '连接已删除')
      loadConnections()
      if (selectedConnection?.id === connId) {
        setSelectedConnection(null)
      }
    } catch {
      // callAPI handles error display
    }
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

  // 预览选中表
  const handlePreview = async () => {
    if (!selectedConnection || selectedTables.size === 0) return

    setLoading(true)
    const infos: TableInfo[] = []
    for (const tableName of selectedTables) {
      const info = await loadTableInfo(selectedConnection.id, tableName)
      if (info) infos.push(info)
    }
    setTableInfos(infos)
    setViewMode('preview')
    setLoading(false)
  }

  // 导出文档
  const handleExportWithDialog = async () => {
    if (!selectedConnection || selectedTables.size === 0) return

    try {
      const outputDir = await window.pluginAPI.open_folder_dialog('选择导出目录')
      if (!outputDir) return

      setExporting(true)
      const result = await callAPI<{ success: boolean; files?: string[]; message?: string }>('export_docs', {
        connection_id: selectedConnection.id,
        tables: Array.from(selectedTables),
        output_dir: outputDir,
        format: exportFormat,
        template: exportTemplate
      })

      if (result.success) {
        showToast('success', `导出成功! 共 ${result.files?.length || 0} 个文件`)
        setShowExportPanel(false)
      } else {
        showToast('error', '导出失败: ' + (result.message || '未知错误'))
      }
    } catch (e) {
      showToast('error', '导出失败: ' + (e instanceof Error ? e.message : '未知错误'))
    } finally {
      setExporting(false)
    }
  }

  return (
    <div className="app">
      <header className="header">
        <h1>📊 数据库文档生成器</h1>
        <div className="steps">
          <div className={`step ${viewMode === 'connections' ? 'active' : 'completed'}`}
            onClick={() => setViewMode('connections')}>
            <span className="step-number">1</span>
            <span className="step-label">连接管理</span>
          </div>
          <div className="step-line"></div>
          <div className={`step ${viewMode === 'tables' ? 'active' : selectedConnection ? 'completed' : ''}`}
            onClick={() => selectedConnection && setViewMode('tables')}>
            <span className="step-number">2</span>
            <span className="step-label">选择表</span>
          </div>
          <div className="step-line"></div>
          <div className={`step ${viewMode === 'preview' ? 'active' : ''}`}>
            <span className="step-number">3</span>
            <span className="step-label">预览 & 导出</span>
          </div>
        </div>
      </header>

      {error && <div className="error">{error}</div>}
      {loading && <div className="loading">加载中...</div>}

      <main className="main">
        {viewMode === 'connections' && (
          <div className="connections-view">
            <div className="connections-list">
              <h2>已保存的连接</h2>
              {connections.length > 0 && (
                <input
                  className="search-input"
                  type="text"
                  placeholder="搜索连接..."
                  value={connectionSearch}
                  onChange={(e) => setConnectionSearch(e.target.value)}
                />
              )}
              {filteredConnections.length === 0 ? (
                <p className="empty">{connections.length === 0 ? '暂无保存的连接配置' : '没有匹配的连接'}</p>
              ) : (
                <ul>
                  {filteredConnections.map((conn) => (
                    <li key={conn.id} className="connection-item">
                      <div className="connection-info">
                        <span className="connection-name">{conn.name}</span>
                        <span className="connection-type">{conn.db_type.toUpperCase()}</span>
                        <span className="connection-host">{conn.host}:{conn.port}</span>
                        {testResult && testResult.id === conn.id && (
                          <span className={`test-result ${testResult.success ? 'test-success' : 'test-fail'}`}>
                            {testResult.success ? '\u2713' : '\u2717'}
                          </span>
                        )}
                      </div>
                      <div className="connection-actions">
                        <button onClick={() => handleSelectConnection(conn)}>选择</button>
                        <button onClick={() => testConnection(conn)} disabled={testingConnection}>
                          {testingConnection ? '测试中...' : '测试连接'}
                        </button>
                        <button className="btn-danger" onClick={() => handleDeleteConnection(conn.id)}>删除</button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="connection-form">
              <h2>新建连接</h2>
              <ConnectionForm
                onSave={async (config) => {
                  await callAPI('save_connection', config as Record<string, unknown>)
                  loadConnections()
                }}
                onTest={testConnection}
              />
            </div>
          </div>
        )}

        {viewMode === 'tables' && selectedConnection && (
          <div className="tables-view">
            <div className="tables-header">
              <h2>{selectedConnection.name} - 选择要导出的表</h2>
              <button onClick={toggleSelectAll}>
                {selectedTables.size === tables.length ? '取消全选' : '全选'}
              </button>
            </div>
            <div className="tables-toolbar">
              <input
                className="search-input"
                type="text"
                placeholder="搜索表名..."
                value={tableSearch}
                onChange={(e) => setTableSearch(e.target.value)}
              />
              <div className="batch-actions">
                <input
                  className="prefix-input"
                  type="text"
                  placeholder="表前缀"
                  value={prefixFilter}
                  onChange={(e) => setPrefixFilter(e.target.value)}
                />
                <button onClick={selectByPrefix} disabled={!prefixFilter}>按前缀选择</button>
                <button onClick={invertSelection}>反选</button>
              </div>
            </div>
            <div className="tables-grid">
              {filteredTables.map((table) => (
                <div
                  key={table}
                  className={`table-item ${selectedTables.has(table) ? 'selected' : ''}`}
                  onClick={() => toggleTableSelection(table)}
                >
                  <input
                    type="checkbox"
                    checked={selectedTables.has(table)}
                    onChange={() => toggleTableSelection(table)}
                  />
                  <span>{table}</span>
                </div>
              ))}
            </div>
            <div className="tables-actions">
              <button onClick={() => setViewMode('connections')}>返回</button>
              <button
                onClick={handlePreview}
                disabled={selectedTables.size === 0}
              >
                预览选中 ({selectedTables.size})
              </button>
            </div>
          </div>
        )}

        {viewMode === 'preview' && (
          <div className="preview-view">
            <div className="preview-header">
              <h2>预览 - {tableInfos.length} 张表</h2>
              <div className="preview-actions">
                <button onClick={() => setViewMode('tables')}>返回选择</button>
                <button onClick={() => setShowExportPanel(true)} className="primary">导出文档</button>
              </div>
            </div>
            <div className="preview-content">
              {tableInfos.map((table) => (
                <TablePreview key={table.name} table={table} />
              ))}
            </div>
          </div>
        )}

        {showExportPanel && (
          <div className="modal-overlay" onClick={() => setShowExportPanel(false)}>
            <div className="export-panel" onClick={(e) => e.stopPropagation()}>
              <h3>导出配置</h3>

              <div className="form-group">
                <label>导出格式</label>
                <div className="radio-group">
                  <label className="radio-item">
                    <input type="radio" name="format" value="markdown"
                      checked={exportFormat === 'markdown'}
                      onChange={() => setExportFormat('markdown')} />
                    <span>Markdown</span>
                  </label>
                  <label className="radio-item">
                    <input type="radio" name="format" value="word"
                      checked={exportFormat === 'word'}
                      onChange={() => setExportFormat('word')} />
                    <span>Word</span>
                  </label>
                  <label className="radio-item">
                    <input type="radio" name="format" value="pdf"
                      checked={exportFormat === 'pdf'}
                      onChange={() => setExportFormat('pdf')} />
                    <span>PDF</span>
                  </label>
                </div>
              </div>

              <div className="form-group">
                <label>模板风格</label>
                <div className="radio-group">
                  <label className="radio-item">
                    <input type="radio" name="template" value="simple"
                      checked={exportTemplate === 'simple'}
                      onChange={() => setExportTemplate('simple')} />
                    <span>简洁</span>
                  </label>
                  <label className="radio-item">
                    <input type="radio" name="template" value="detailed"
                      checked={exportTemplate === 'detailed'}
                      onChange={() => setExportTemplate('detailed')} />
                    <span>详细</span>
                  </label>
                </div>
              </div>

              <div className="form-actions">
                <button onClick={() => setShowExportPanel(false)}>取消</button>
                <button className="primary" onClick={handleExportWithDialog} disabled={exporting}>
                  {exporting ? '导出中...' : '选择目录并导出'}
                </button>
              </div>
            </div>
          </div>
        )}
      </main>

      {toasts.length > 0 && (
        <div className="toast-container">
          {toasts.map(toast => (
            <div key={toast.id} className={`toast toast-${toast.type}`}>
              {toast.type === 'success' && '\u2713 '}
              {toast.type === 'error' && '\u2717 '}
              {toast.message}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// 连接表单组件
function ConnectionForm({
  onSave,
  onTest,
}: {
  onSave: (config: Partial<ConnectionConfig>) => Promise<void>
  onTest: (config: Partial<ConnectionConfig>) => Promise<void>
}) {
  const [config, setConfig] = useState<Partial<ConnectionConfig>>({
    name: '',
    db_type: 'mysql',
    host: 'localhost',
    port: 3306,
    database: '',
    username: 'root',
    password: '',
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await onSave(config)
  }

  const handleTest = async () => {
    await onTest(config)
  }

  return (
    <form onSubmit={handleSubmit} className="form">
      <div className="form-group">
        <label>连接名称</label>
        <input
          value={config.name}
          onChange={(e) => setConfig({ ...config, name: e.target.value })}
          placeholder="例如: 生产环境"
          required
        />
      </div>

      <div className="form-group">
        <label>数据库类型</label>
        <select
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
          <label>主机地址</label>
          <input
            value={config.host}
            onChange={(e) => setConfig({ ...config, host: e.target.value })}
            placeholder="localhost"
          />
        </div>
        <div className="form-group">
          <label>端口</label>
          <input
            type="number"
            value={config.port}
            onChange={(e) => setConfig({ ...config, port: parseInt(e.target.value) })}
          />
        </div>
      </div>

      <div className="form-group">
        <label>数据库名</label>
        <input
          value={config.database}
          onChange={(e) => setConfig({ ...config, database: e.target.value })}
          placeholder="database_name"
          required
        />
      </div>

      <div className="form-group">
        <label>用户名</label>
        <input
          value={config.username}
          onChange={(e) => setConfig({ ...config, username: e.target.value })}
          placeholder="root"
        />
      </div>

      <div className="form-group">
        <label>密码</label>
        <input
          type="password"
          value={config.password}
          onChange={(e) => setConfig({ ...config, password: e.target.value })}
          placeholder="••••••••"
        />
      </div>

      <div className="form-actions">
        <button type="button" onClick={handleTest}>测试连接</button>
        <button type="submit" className="primary">保存</button>
      </div>
    </form>
  )
}

// 表预览组件
function TablePreview({ table }: { table: TableInfo }) {
  return (
    <div className="table-preview">
      <h3>{table.name}</h3>
      {table.comment && <p className="table-comment">{table.comment}</p>}

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
              <td>{col.name}</td>
              <td>
                {col.data_type.toUpperCase()}
                {col.max_length && `(${col.max_length})`}
              </td>
              <td>{col.is_nullable ? '是' : '否'}</td>
              <td>{col.is_primary_key ? '是' : ''}</td>
              <td>{col.default_value || '-'}</td>
              <td>{col.comment || '-'}</td>
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
              </tr>
            </thead>
            <tbody>
              {table.indexes.map((idx) => (
                <tr key={idx.name}>
                  <td>{idx.name}</td>
                  <td>{idx.columns.join(', ')}</td>
                  <td>{idx.is_unique ? '是' : '否'}</td>
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
