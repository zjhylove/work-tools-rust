import { useState, useEffect, useCallback } from 'react'
import './App.css'
import { ApiDocConfig, ControllerInfo, ApiInfo, ExportHistory, ViewMode } from './types'
import StepHeader from './components/StepHeader'
import ConfigView from './components/ConfigView'
import SelectView from './components/SelectView'
import PreviewView from './components/PreviewView'

declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>
      open_folder_dialog: (title?: string) => Promise<string | null>
      open_file_dialog: (title?: string, filters?: { name: string; extensions: string[] }[]) => Promise<string | null>
    }
  }
}

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

  useEffect(() => {
    const waitForAPI = setInterval(() => {
      if (window.pluginAPI) {
        clearInterval(waitForAPI)
        setApiReady(true)
      }
    }, 100)
    return () => clearInterval(waitForAPI)
  }, [])

  useEffect(() => {
    if (!apiReady) return
    const init = async () => {
      try {
        const savedConfig = await callAPI<ApiDocConfig | null>('load_config')
        if (savedConfig) setConfig({ ...defaultConfig, ...savedConfig })
        const h = await callAPI<ExportHistory[]>('get_export_history')
        setHistory(h)
      } catch {
        // First run
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

  const toggleExpandClass = (cn: string) => {
    setExpandedClasses(prev => {
      const next = new Set(prev)
      if (next.has(cn)) next.delete(cn)
      else next.add(cn)
      return next
    })
  }

  const selectAll = () => {
    const all = new Set<string>()
    controllers.forEach(c => c.methods.forEach(m => all.add(`${c.class_name}::${m.method_name}`)))
    setSelectedMethods(all)
  }

  const deselectAll = () => setSelectedMethods(new Set())

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
      const h = await callAPI<ExportHistory[]>('get_export_history')
      setHistory(h)
    } catch {
      // error handled
    } finally {
      setLoading(false)
    }
  }

  const toggleApi = (path: string) => {
    setExpandedApis(prev => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  return (
    <div className="app">
      {toast && <div className={`toast toast--${toast.type}`}>{toast.message}</div>}
      <StepHeader view={view} />
      {error && <div className="error-banner">{error}</div>}

      {view === 'config' && (
        <ConfigView
          config={config}
          setConfig={setConfig}
          loading={loading}
          history={history}
          onScan={handleScan}
          onOpenJar={handleOpenJar}
        />
      )}

      {view === 'select' && (
        <SelectView
          controllers={controllers}
          selectedMethods={selectedMethods}
          expandedClasses={expandedClasses}
          searchFilter={searchFilter}
          apis={apis}
          expandedApis={expandedApis}
          loading={loading}
          exportFormats={exportFormats}
          outputDir={outputDir}
          onBack={() => setView('config')}
          onToggleMethod={toggleMethod}
          onToggleClass={toggleClass}
          onToggleExpand={toggleExpandClass}
          onSelectAll={selectAll}
          onDeselectAll={deselectAll}
          onSearchChange={setSearchFilter}
          onParse={handleParseDetails}
          onToggleApi={toggleApi}
          onFormatChange={setExportFormats}
          onOutputDirChange={setOutputDir}
          onOpenOutputDir={handleOpenOutputDir}
          onExport={handleExport}
        />
      )}

      {view === 'preview' && (
        <PreviewView
          outputFiles={outputFiles}
          onBack={() => { setView('select'); setOutputFiles([]) }}
          onRestart={() => { setView('config'); setApis([]); setControllers([]); setSelectedMethods(new Set()) }}
        />
      )}
    </div>
  )
}

export default App
