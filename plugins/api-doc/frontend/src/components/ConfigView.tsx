import { ApiDocConfig, ExportHistory } from '../types'
import HistoryList from './HistoryList'

interface Props {
  config: ApiDocConfig
  setConfig: React.Dispatch<React.SetStateAction<ApiDocConfig>>
  loading: boolean
  history: ExportHistory[]
  onScan: () => void
  onOpenJar: () => void
}

export default function ConfigView({ config, setConfig, loading, history, onScan, onOpenJar }: Props) {
  return (
    <div className="view-container view-container--centered">
      <div className="card card--config">
        <div className="card-title">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <h2>JAR 文件配置</h2>
        </div>

        <div className="form-group">
          <label className="form-label">Spring Boot JAR 路径</label>
          <div className="input-row">
            <input
              type="text"
              className="form-input"
              value={config.source_jar_path}
              onChange={e => setConfig(prev => ({ ...prev, source_jar_path: e.target.value }))}
              placeholder="选择或输入 JAR 文件路径"
            />
            <button onClick={onOpenJar} className="btn btn--outline">浏览</button>
          </div>
        </div>

        <div className="form-group">
          <label className="form-label">服务名称</label>
          <input
            type="text"
            className="form-input"
            value={config.service_name}
            onChange={e => setConfig(prev => ({ ...prev, service_name: e.target.value }))}
            placeholder="如: user-service"
          />
        </div>

        <div className="form-group">
          <label className="form-checkbox-label">
            <input
              type="checkbox"
              checked={config.auto_scan_dependencies}
              onChange={e => setConfig(prev => ({ ...prev, auto_scan_dependencies: e.target.checked }))}
              className="form-checkbox"
            />
            <span>自动扫描依赖 JAR</span>
          </label>
        </div>

        <div className="form-group">
          <label className="form-label">依赖 JAR 前缀 (逗号分隔)</label>
          <input
            type="text"
            className="form-input"
            value={config.dependency_jars.join(',')}
            onChange={e => setConfig(prev => ({
              ...prev,
              dependency_jars: e.target.value.split(',').map(s => s.trim()).filter(Boolean),
            }))}
            placeholder="如: my-company,shared-lib"
            disabled={config.auto_scan_dependencies}
          />
        </div>

        <button onClick={onScan} className="btn btn--primary btn--block" disabled={loading}>
          {loading ? (
            <>
              <span className="spinner" />
              扫描中...
            </>
          ) : '扫描 Controller'}
        </button>
      </div>

      <HistoryList history={history} />
    </div>
  )
}
