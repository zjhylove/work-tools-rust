#!/bin/bash

echo "======================================"
echo "   插件迁移验证测试"
echo "======================================"
echo ""
echo "测试所有迁移后的插件包"
echo ""

PLUGINS=("password-manager" "auth-plugin")

for PLUGIN in "${PLUGINS[@]}"; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "测试插件: $PLUGIN"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    ZIP_FILE="${PLUGIN}.wtplugin.zip"

    # 检查插件包是否存在
    if [ ! -f "$ZIP_FILE" ]; then
        echo "❌ 错误: 找不到 $ZIP_FILE"
        continue
    fi

    echo "✅ 找到插件包: $ZIP_FILE"
    echo ""

    # 验证插件包结构
    echo "验证插件包结构..."

    # 检查 manifest.json
    if unzip -l "$ZIP_FILE" | grep -q "manifest.json"; then
        echo "   ✅ manifest.json 存在"
    else
        echo "   ❌ 缺少 manifest.json"
        continue
    fi

    # 检查动态库
    if [[ "$PLUGIN" == "password-manager" ]]; then
        LIB_NAME="libpassword_manager.dylib"
    elif [[ "$PLUGIN" == "auth-plugin" ]]; then
        LIB_NAME="libauth_plugin.dylib"
    fi

    if unzip -l "$ZIP_FILE" | grep -q "$LIB_NAME"; then
        echo "   ✅ $LIB_NAME 存在"
    else
        echo "   ❌ 缺少动态库文件"
        continue
    fi

    # 检查前端资源
    if unzip -l "$ZIP_FILE" | grep -q "assets/index.html"; then
        echo "   ✅ assets/index.html 存在"
    else
        echo "   ❌ 缺少前端资源"
        continue
    fi

    if unzip -l "$ZIP_FILE" | grep -q "assets/main.js"; then
        echo "   ✅ assets/main.js 存在"
    else
        echo "   ❌ 缺少 main.js"
        continue
    fi

    if unzip -l "$ZIP_FILE" | grep -q "assets/styles.css"; then
        echo "   ✅ assets/styles.css 存在"
    else
        echo "   ❌ 缺少 styles.css"
        continue
    fi

    echo ""
    echo "📦 插件包信息:"
    echo "   大小: $(du -h "$ZIP_FILE" | cut -f1)"
    echo "   文件数: $(unzip -l "$ZIP_FILE" | tail -1 | awk '{print $2}')"
    echo ""

    echo "✅ $PLUGIN 插件包验证通过"
    echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "测试完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "📋 插件包汇总:"
echo ""
ls -lh *.wtplugin.zip 2>/dev/null | awk '{print "   " $9 " - " $5}' || echo "   未找到插件包"
echo ""

echo "======================================"
echo "   下一步:导入测试"
echo "======================================"
echo ""
echo "🚀 启动应用并导入插件:"
echo ""
echo "1. cd tauri-app"
echo "2. npm run tauri dev"
echo "3. 点击插件商店按钮 (🧩)"
echo ""
echo "📪 按顺序导入插件:"
echo "   - password-manager.wtplugin.zip"
echo "   - auth-plugin.wtplugin.zip"
echo ""
echo "✅ 验证每个插件:"
echo "   - 插件出现在商店列表"
echo "   - 侧边栏显示插件菜单"
echo "   - 点击菜单能看到完整界面"
echo "   - 功能正常工作"
echo ""
echo "======================================"
echo ""
echo "准备开始测试? 按回车键继续..."
read
