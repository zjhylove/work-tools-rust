import { ApiDocConfig } from '../types'

interface Props {
  config: ApiDocConfig
  setConfig: React.Dispatch<React.SetStateAction<ApiDocConfig>>
  loading: boolean
  onScan: () => void
  onOpenJar: () => void
}

export default function ConfigView({ config, setConfig, loading, onScan, onOpenJar }: Props) {
  const jarName = config.source_jar_path
    ? config.source_jar_path.split(/[/\\]/).pop()
    : ''

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
          <label className="form-label">Spring Boot JAR 文件</label>
          {config.source_jar_path ? (
            <div className="jar-path-display">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <span>{jarName}</span>
              <button
                onClick={onOpenJar}
                className="btn btn--ghost btn--xs"
                style={{ marginLeft: 'auto', flexShrink: 0 }}
              >
                更换
              </button>
            </div>
          ) : (
            <button onClick={onOpenJar} className="btn btn--primary btn--block">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              选择 JAR 文件
            </button>
          )}
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

        <button
          onClick={onScan}
          className="btn btn--primary btn--block"
          disabled={loading || !config.source_jar_path}
        >
          {loading ? (
            <>
              <span className="spinner" />
              扫描中...
            </>
          ) : '扫描 Controller'}
        </button>
      </div>
    </div>
  )
}
