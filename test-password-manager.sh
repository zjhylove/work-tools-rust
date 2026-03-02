#!/bin/bash

echo "======================================"
echo "   密码管理器插件测试脚本"
echo "======================================"
echo ""

# 检查插件包是否存在
if [ ! -f "password-manager.wtplugin.zip" ]; then
    echo "❌ 错误: 找不到 password-manager.wtplugin.zip"
    echo "请确保在项目根目录运行此脚本"
    exit 1
fi

echo "✅ 找到密码管理器插件包: password-manager.wtplugin.zip"
echo ""

# 验证插件包结构
echo "1️⃣  验证插件包结构..."
if unzip -l password-manager.wtplugin.zip | grep -q "manifest.json"; then
    echo "   ✅ manifest.json 存在"
else
    echo "   ❌ 缺少 manifest.json"
    exit 1
fi

if unzip -l password-manager.wtplugin.zip | grep -q "libpassword_manager.dylib"; then
    echo "   ✅ libpassword_manager.dylib 存在"
else
    echo "   ❌ 缺少动态库文件"
    exit 1
fi

if unzip -l password-manager.wtplugin.zip | grep -q "assets/index.html"; then
    echo "   ✅ assets/index.html 存在"
else
    echo "   ❌ 缺少前端资源"
    exit 1
fi

echo ""
echo "2️⃣  显示 manifest.json 内容..."
echo ""
unzip -p password-manager.wtplugin.zip manifest.json | jq '.'
echo ""

echo "3️⃣  检查现有插件安装..."
if [ -d "$HOME/.worktools/plugins" ]; then
    echo "   已安装的插件:"
    ls -1 "$HOME/.worktools/plugins" 2>/dev/null | sed 's/^/   - /'
else
    echo "   插件目录不存在,这是正常的(首次安装)"
fi
echo ""

echo "======================================"
echo "   手动测试步骤"
echo "======================================"
echo ""
echo "📋 插件包信息:"
echo "   位置: $(pwd)/password-manager.wtplugin.zip"
echo "   大小: $(du -h password-manager.wtplugin.zip | cut -f1)"
echo ""
echo "🚀 启动应用:"
echo "   cd tauri-app"
echo "   npm run tauri dev"
echo ""
echo "📪 导入插件:"
echo "   1. 点击侧边栏底部的插件商店按钮 (🧩)"
echo "   2. 点击'导入插件'按钮"
echo "   3. 在文件对话框中选择 password-manager.wtplugin.zip"
echo "   4. 等待导入完成"
echo ""
echo "✅ 验证安装:"
echo "   - 插件应该出现在商店列表中"
echo "   - 插件应该显示'已安装'状态"
echo "   - 侧边栏应该显示'密码管理器'菜单项 (🔐)"
echo ""
echo "🎨 测试插件功能:"
echo "   1. 点击侧边栏的'密码管理器'"
echo "   2. 应该看到密码管理界面"
echo "   3. 点击'添加密码'按钮"
echo "   4. 填写表单并保存"
echo "   5. 验证密码是否正确保存"
echo "   6. 测试复制和删除功能"
echo ""
echo "🔍 验证文件安装:"
echo "   ls -la ~/.worktools/plugins/password-manager/"
echo ""
echo "📝 查看注册表:"
echo "   cat ~/.worktools/config/installed-plugins.json | jq '.'"
echo ""
echo "🗑️  卸载插件:"
echo "   1. 重新打开插件商店"
echo "   2. 找到'密码管理器'"
echo "   3. 点击'卸载'按钮"
echo "   4. 验证插件从侧边栏消失"
echo ""
echo "======================================"
echo ""
echo "准备好后按回车键继续..."
read
