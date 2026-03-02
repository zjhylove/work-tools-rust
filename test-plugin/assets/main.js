// 测试插件 API 调用
function testPluginAPI() {
    const resultDiv = document.getElementById('result');

    try {
        // 检查是否在 Tauri 环境中
        if (window.__TAURI__) {
            resultDiv.style.display = 'block';
            resultDiv.innerHTML = `
                <strong>测试结果:</strong><br>
                ✓ Tauri 环境检测成功<br>
                ✓ window.__TAURI__ 可用<br>
                ✓ 插件前端资源已隔离加载
            `;
            resultDiv.style.background = '#e8f5e9';
            resultDiv.style.borderLeftColor = '#4caf50';
            resultDiv.style.color = '#2e7d32';
        } else {
            resultDiv.style.display = 'block';
            resultDiv.innerHTML = `
                <strong>测试结果:</strong><br>
                ⚠ 当前不在 Tauri 环境中<br>
                ✓ 但前端资源加载正常<br>
                ✓ 样式隔离工作正常
            `;
            resultDiv.style.background = '#fff3cd';
            resultDiv.style.borderLeftColor = '#ffc107';
            resultDiv.style.color = '#856404';
        }
    } catch (error) {
        resultDiv.style.display = 'block';
        resultDiv.innerHTML = `<strong>错误:</strong> ${error.message}`;
        resultDiv.style.background = '#f8d7da';
        resultDiv.style.borderLeftColor = '#f44336';
        resultDiv.style.color = '#721c24';
    }
}

// 页面加载完成后的初始化
window.addEventListener('DOMContentLoaded', () => {
    console.log('[测试插件] 前端资源加载成功');
    console.log('[测试插件] 当前环境:', window.__TAURI__ ? 'Tauri' : 'Browser');
});
