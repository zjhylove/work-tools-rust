import fs from 'fs';
import path from 'path';

interface ComponentMapping {
  signal: string;
  state: string;
  effect: string;
  show: string;
  for: string;
}

const SOLID_TO_REACT: ComponentMapping = {
  signal: 'const [value, setValue] = useState(initial);',
  state: 'const [value, setValue] = useState(initial);',
  effect: 'useEffect(() => { /* ... */ }, []);',
  show: '{condition && <Component />}',
  for: '{items.map(item => <Component key={item.id} {...item} />)}',
};

export function migrateSolidToReact(code: string): string {
  let result = code;

  // 替换 createSignal
  result = result.replace(
    /const (\w+)\s*=\s*createSignal\(([^)]+)\)/g,
    'const [$1, set$1] = useState($2)'
  );

  // 替换 signal() 调用
  result = result.replace(
    /(\w+)\(\)/g,
    '$1'
  );

  // 替换 setSignal
  result = result.replace(
    /set(\w+)\(([^)]+)\)/g,
    'set$1($2)'
  );

  // 替换 Show 组件
  result = result.replace(
    /<Show when={([^}]+)}>\s*(.*?)\s*<\/Show>/gs,
    '{$1 &&\n        $2\n      }'
  );

  // 替换 For 组件
  result = result.replace(
    /<For each={(\w+)\(\)}>\s*{(.*?)=>\s*<([^>]+)([^>]*)>\s*(.*?)\s*<\/\2>\s*<\/For>/gs,
    '{$1.map($3 => (\n        <$2$4 {...$3}>\n          $5\n        </$2>\n      ))}'
  );

  return result;
}

// CLI 工具
if (require.main === module) {
  const inputFile = process.argv[2];
  const outputFile = process.argv[3];

  if (!inputFile) {
    console.error('用法: ts-node migrate-component.ts <input-file> [output-file]');
    process.exit(1);
  }

  const code = fs.readFileSync(inputFile, 'utf-8');
  const migrated = migrateSolidToReact(code);

  if (outputFile) {
    fs.writeFileSync(outputFile, migrated);
    console.log(`✅ 迁移完成: ${outputFile}`);
  } else {
    console.log(migrated);
  }
}
