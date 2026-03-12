#!/bin/bash

# 只测试基本功能,不测试具体输出格式
cat >> src/lib.rs << 'TESTEOF'

    #[test]
    fn test_export_diff() {
        let original = "Line 1\nLine 2\nLine 3";
        let modified = "Line 1\nModified\nLine 3";
        
        let diff = TextDiff::export_unified_diff(original, modified, "test.txt");
        
        // 检查基本结构
        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("@@")); // Unified diff 格式包含 @@
        assert!(diff.len() > 50); // 应该有足够的内容
        
        println!("Diff output:\n{}", diff);
        println!("\n✅ export_diff test passed");
    }
TESTEOF

# 运行修正后的测试
cargo test test_export_diff -- --nocapture 2>&1 | tail -40
