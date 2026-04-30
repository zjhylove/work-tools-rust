export interface ConnectionConfig {
  id: string;
  provider: string;
  name: string;
  region: string;
  bucket: string;
  endpoint?: string;
}

export interface ObjectInfo {
  key: string;
  size: number;
  last_modified: string;
  etag: string;
  is_dir: boolean;
}

export interface ListObjectsResult {
  objects: ObjectInfo[];
  prefixes: string[];
}
