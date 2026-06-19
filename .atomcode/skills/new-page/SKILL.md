# new-page

页面脚手架。创建新的 AtlasForge 页面组件，遵循项目现有的页面结构和风格。

## 用法

```
/new-page <PageName>
```

## 步骤

1. 查看现有页面列表和路由配置：
   - 读取 `src/App.tsx` 了解路由结构
   - 读取一个现有页面（如 `src/pages/Dashboard.tsx`）了解风格

2. 创建页面文件 `src/pages/<PageName>.tsx`，遵循以下模板：
   ```tsx
   import { useState } from 'react';

   export default function <PageName>() {
     return (
       <div className="p-6">
         <h1 className="text-2xl font-bold mb-4"><PageName></h1>
         {/* 页面内容 */}
       </div>
     );
   }
   ```

3. 在 `src/App.tsx` 中添加路由：
   - 导入新页面组件
   - 在 `<Routes>` 中添加 `<Route path="/<page-name>" element={<<PageName> />} />`

4. 如果需要，在 `src/components/Layout.tsx` 的导航中添加入口

5. 运行类型检查确认无错误：
   ```bash
   npx tsc --noEmit
   ```

## 约定

- 页面组件使用默认导出（`export default function`）
- 页面名使用 PascalCase
- 路由路径使用 kebab-case
- 使用 lucide-react 图标（项目已安装）
- 使用 Tailwind CSS 类名（项目风格）
- 页面顶层容器使用 `className="p-6"`
- 标题使用 `className="text-2xl font-bold mb-4"`
