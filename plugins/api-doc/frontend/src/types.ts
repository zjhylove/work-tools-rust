export interface ApiDocConfig {
  source_jar_path: string
  service_name: string
  dependency_jars: string[]
  auto_scan_dependencies: boolean
}

export interface MethodInfo {
  method_name: string
  http_method: string
  path: string
  api_name: string
}

export interface ControllerInfo {
  class_name: string
  class_path: string
  methods: MethodInfo[]
}

export interface ApiField {
  field_name: string
  field_type: string
  required: string
  field_length: string
  comment: string
  example_value: string
}

export interface NodeInfo {
  node_name: string
  node_desc: string
  resp_fields: ApiField[]
}

export interface ApiInfo {
  api_name: string
  http_method: string
  service_name: string
  business_module: string
  method_name: string
  version: string
  full_path: string
  req_fields: ApiField[]
  req_nodes: NodeInfo[]
  req_example: string
  resp_nodes: NodeInfo[]
  resp_example: string
}

export interface ExportHistory {
  id: string
  service_name: string
  api_count: number
  formats: string[]
  output_path: string
  exported_at: string
}

export type ViewMode = 'config' | 'select' | 'preview'

export function httpMethodColor(method: string): string {
  switch (method) {
    case 'GET': return 'method-get'
    case 'POST': return 'method-post'
    case 'PUT': return 'method-put'
    case 'DELETE': return 'method-delete'
    case 'PATCH': return 'method-patch'
    default: return ''
  }
}
