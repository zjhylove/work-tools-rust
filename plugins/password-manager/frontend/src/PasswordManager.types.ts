/**
 * 密码管理器类型定义
 */

export interface PasswordEntry {
  id?: string;
  title: string;
  username: string;
  password: string;
  url?: string;
  notes?: string;
  created_at?: number;
  updated_at?: number;
}

export interface PasswordFormData {
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
}
