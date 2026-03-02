#!/bin/bash

echo "=== 插件商店功能测试 ==="
echo ""

# 1. 验证插件包结构
echo "1. 验证插件包结构..."
unzip -l test-plugin.wtplugin.zip | grep -E "(manifest.json|libtest_plugin.dylib|assets/index.html)"

if [ $? -eq 0 ]; then
    echo "✓ 插件包结构正确"
else
    echo "✗ 插件包结构错误"
    exit 1
fi

echo ""
echo "2. 插件包内容:"
echo ""
echo "文件列表:"
unzip -l test-plugin.wtplugin.zip

echo ""
echo "3. manifest.json 内容:"
echo ""
unzip -p test-plugin.wtplugin.zip test-plugin/manifest.json | jq '.'

echo ""
echo "=== 验证完成 ==="
echo ""
echo "插件包路径: $(pwd)/test-plugin.wtplugin.zip"
echo ""
echo "下一步:"
echo "1. 启动应用: cd tauri-app && npm run tauri dev"
echo "2. 点击插件商店按钮 (🧩)"
echo "3. 点击'导入插件'"
echo "4. 选择 test-plugin.wtplugin.zip"
echo "5. 验证插件是否成功安装并出现在菜单中"
