import { Component, createSignal, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import './Dialog.css';

interface SettingsProps {
  onClose: () => void;
}

const Settings: Component<SettingsProps> = (props) => {
  const [theme, setTheme] = createSignal('light');
  const [autoStart, setAutoStart] = createSignal(false);
  const [minimizeToTray, setMinimizeToTray] = createSignal(true);

  onMount(async () => {
    try {
      const config = await invoke<any>('get_app_config');
      setTheme(config.theme || 'light');
      setAutoStart(config.settings?.autoStart || false);
      setMinimizeToTray(config.settings?.minimizeToTray || true);
    } catch (error) {
      console.error('Failed to load config:', error);
    }
  });

  const handleSave = async () => {
    try {
      await invoke('set_app_config', {
        config: {
          theme: theme(),
          settings: {
            autoStart: autoStart(),
            minimizeToTray: minimizeToTray(),
          },
        },
      });
      props.onClose();
    } catch (error) {
      console.error('Failed to save config:', error);
      alert('保存配置失败');
    }
  };

  return (
    <div class="dialog-overlay" onClick={props.onClose}>
      <div class="dialog-content" onClick={(e) => e.stopPropagation()}>
        <div class="dialog-header">
          <h2>设置</h2>
          <button class="dialog-close" onClick={props.onClose}>
            ✕
          </button>
        </div>
        <div class="dialog-body">
          <div class="settings-section">
            <h3>外观</h3>
            <div class="settings-item">
              <label>主题</label>
              <select
                value={theme()}
                onInput={(e) => setTheme(e.currentTarget.value)}
              >
                <option value="light">浅色</option>
                <option value="dark">深色</option>
              </select>
            </div>
          </div>

          <div class="settings-section">
            <h3>通用</h3>
            <div class="settings-item">
              <label>开机自动启动</label>
              <input
                type="checkbox"
                checked={autoStart()}
                onInput={(e) => setAutoStart(e.currentTarget.checked)}
              />
            </div>
            <div class="settings-item">
              <label>最小化到托盘</label>
              <input
                type="checkbox"
                checked={minimizeToTray()}
                onInput={(e) => setMinimizeToTray(e.currentTarget.checked)}
              />
            </div>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="button-secondary" onClick={props.onClose}>
            取消
          </button>
          <button class="button-primary" onClick={handleSave}>
            保存
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
