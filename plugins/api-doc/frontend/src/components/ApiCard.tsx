import { ApiInfo, httpMethodColor } from '../types'

interface Props {
  api: ApiInfo
  isExpanded: boolean
  onToggle: () => void
  searchQuery?: string
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

export default function ApiCard({ api, isExpanded, onToggle, searchQuery = '' }: Props) {
  const query = searchQuery.trim()

  return (
    <div className={`api-card ${isExpanded ? 'api-card--expanded' : ''}`}>
      <div className="api-card-header" onClick={onToggle}>
        <span className={`method-badge method-badge--pill ${httpMethodColor(api.http_method)}`}>
          {highlightMatch(api.http_method, query)}
        </span>
        <span className="api-card-path">{highlightMatch(api.full_path, query)}</span>
        <span className="api-card-name">{highlightMatch(api.api_name, query)}</span>
        <span className={`expand-arrow ${isExpanded ? 'expanded' : ''}`}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </span>
      </div>

      {isExpanded && (
        <div className="api-card-body">
          {api.version && (
            <div className="api-meta-row">
              <span className="api-meta-label">版本</span>
              <span className="api-meta-value">{api.version}</span>
              {api.business_module && (
                <>
                  <span className="api-meta-label">模块</span>
                  <span className="api-meta-value">{api.business_module}</span>
                </>
              )}
            </div>
          )}

          {api.req_fields.length > 0 && (
            <div className="param-section">
              <div className="param-section-title">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="16 18 22 12 16 6" />
                  <polyline points="8 6 2 12 8 18" />
                </svg>
                请求参数
              </div>
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr><th>字段名</th><th>类型</th><th>必填</th><th>注释</th></tr>
                  </thead>
                  <tbody>
                    {api.req_fields.map(f => (
                      <tr key={f.field_name}>
                        <td><code>{highlightMatch(f.field_name, query)}</code></td>
                        <td>{highlightMatch(f.field_type, query)}</td>
                        <td><span className={`required-tag ${f.required === '是' ? 'required-tag--yes' : 'required-tag--no'}`}>{f.required}</span></td>
                        <td>{highlightMatch(f.comment, query)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {api.req_nodes.map(node => (
                <div key={node.node_name} className="resp-node">
                  <div className="resp-node-header">
                    {highlightMatch(node.node_name, query)}
                    {node.node_desc && <span className="resp-node-desc">({highlightMatch(node.node_desc, query)})</span>}
                  </div>
                  <div className="table-wrap">
                    <table>
                      <thead><tr><th>字段名</th><th>类型</th><th>必填</th><th>注释</th></tr></thead>
                      <tbody>
                        {node.resp_fields.map(f => (
                          <tr key={f.field_name}>
                            <td><code>{highlightMatch(f.field_name, query)}</code></td>
                            <td>{highlightMatch(f.field_type, query)}</td>
                            <td><span className={`required-tag ${f.required === '是' ? 'required-tag--yes' : 'required-tag--no'}`}>{f.required}</span></td>
                            <td>{highlightMatch(f.comment, query)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              ))}
            </div>
          )}

          {api.req_example && (
            <div className="code-section">
              <div className="code-section-header">
                <span className="code-section-label">请求示例</span>
                <span className="code-section-tag">JSON</span>
              </div>
              <pre className="code-block"><code>{api.req_example}</code></pre>
            </div>
          )}

          {api.resp_nodes.length > 0 && (
            <div className="param-section">
              <div className="param-section-title">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
                </svg>
                响应参数
              </div>
              {api.resp_nodes.map(node => (
                <div key={node.node_name} className="resp-node">
                  <div className="resp-node-header">
                    {highlightMatch(node.node_name, query)} {node.node_desc && <span className="resp-node-desc">({highlightMatch(node.node_desc, query)})</span>}
                  </div>
                  <div className="table-wrap">
                    <table>
                      <thead><tr><th>字段名</th><th>类型</th><th>注释</th></tr></thead>
                      <tbody>
                        {node.resp_fields.map(f => (
                          <tr key={f.field_name}>
                            <td><code>{highlightMatch(f.field_name, query)}</code></td>
                            <td>{highlightMatch(f.field_type, query)}</td>
                            <td>{highlightMatch(f.comment, query)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              ))}
            </div>
          )}

          {api.resp_example && (
            <div className="code-section">
              <div className="code-section-header">
                <span className="code-section-label">响应示例</span>
                <span className="code-section-tag">JSON</span>
              </div>
              <pre className="code-block"><code>{api.resp_example}</code></pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
