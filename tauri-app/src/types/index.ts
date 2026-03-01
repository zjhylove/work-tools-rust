/**
 * 工具类型定义
 * 与 shared/types/src/lib.rs 中的 Rust 类型保持同步
 */

// === 插件相关类型 ===

export interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  icon: string;
}

// === UI Schema 类型 ===

export interface ViewSchema {
  fields: UiField[];
}

// 基础字段类型 (所有 UiField 共有)
export interface BaseField {
  type: string;
  label?: string;
  key?: string;
  default?: string | number | boolean;
}

// Input 字段
export interface InputField extends BaseField {
  type: "input";
  label: string;
  key: string;
  placeholder?: string;
  input_type?: "text" | "password" | "email" | "url" | "number";
  required?: boolean;
}

// Number 字段
export interface NumberField extends BaseField {
  type: "number";
  label: string;
  key: string;
  min?: number;
  max?: number;
}

// Button 字段
export interface ButtonField extends BaseField {
  type: "button";
  label: string;
  key: string;
  action?: string;
  icon?: string;
  variant?: "" | "primary" | "secondary" | "danger";
}

// Checkbox 字段
export interface CheckboxField extends BaseField {
  type: "checkbox";
  label: string;
  key: string;
}

// Select 字段
export interface SelectField extends BaseField {
  type: "select";
  label: string;
  key: string;
  options: SelectOption[];
}

export interface SelectOption {
  label: string;
  value: string;
}

// Table 字段 (简单表格)
export interface TableField extends BaseField {
  type: "table";
  label: string;
  columns: string[];
}

// TableList 字段 (高级表格)
export interface TableListField extends BaseField {
  type: "table_list";
  label: string;
  data_binding: string;
  columns: Column[];
  actions?: Action[];
  search_placeholder?: string;
  pagination?: boolean;
}

export interface Column {
  key: string;
  label: string;
  width?: string;
  render?: "password" | "text";
}

export interface Action {
  label: string;
  action: string;
  variant?: string;
}

// Form 字段
export interface FormField extends BaseField {
  type: "form";
  label: string;
  fields: UiField[];
  submit_action: string;
  cancel_action?: string;
  validation?: Record<string, string>;
}

// Dialog 字段
export interface DialogField extends BaseField {
  type: "dialog";
  title: string;
  content: UiField[];
  trigger_action: string;
  width?: string;
  height?: string;
}

// Tabs 字段
export interface TabsField extends BaseField {
  type: "tabs";
  tabs: TabItem[];
  default_tab?: string;
}

export interface TabItem {
  label: string;
  content: UiField[];
}

// Group 字段
export interface GroupField extends BaseField {
  type: "group";
  label: string;
  fields: UiField[];
  collapsible?: boolean;
}

// 联合类型
export type UiField =
  | InputField
  | NumberField
  | ButtonField
  | CheckboxField
  | SelectField
  | TableField
  | TableListField
  | FormField
  | DialogField
  | TabsField
  | GroupField;

// === 数据类型 ===

export type PluginData = Record<string, unknown>;

// === Action 处理类型 ===

export type ActionHandler = (
  action: string,
  data: PluginData,
) => void | Promise<void>;
