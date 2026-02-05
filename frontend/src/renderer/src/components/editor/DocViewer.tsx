import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { useEditorStore } from '../../stores/editorStore'

/**
 * 文档查看器组件
 *
 * 用于在编辑器右侧显示对应的 Markdown 文档或目录总结
 */
export function DocViewer(): JSX.Element {
  const { activeDocContent, selectedDirSummary, activeFile } = useEditorStore()

  // 优先显示目录总结
  if (selectedDirSummary) {
    return (
      <div className="h-full flex flex-col bg-[#1e1e1e]">
        {/* 标题栏 */}
        <div className="h-9 bg-[#252526] flex items-center border-b border-[#3c3c3c] px-3 shrink-0">
          <DocHeaderIcon />
          <span className="text-sm text-gray-300 ml-1.5 truncate">
            {selectedDirSummary.name} - 目录总结
          </span>
        </div>

        {/* 文档内容 */}
        <div className="flex-1 overflow-auto p-4">
          <div className="prose prose-invert prose-sm max-w-none">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={markdownComponents}
            >
              {selectedDirSummary.content}
            </ReactMarkdown>
          </div>
        </div>
      </div>
    )
  }

  // 显示当前文件的文档
  if (activeDocContent && activeFile) {
    return (
      <div className="h-full flex flex-col bg-[#1e1e1e]">
        {/* 标题栏 */}
        <div className="h-9 bg-[#252526] flex items-center border-b border-[#3c3c3c] px-3 shrink-0">
          <DocHeaderIcon />
          <span className="text-sm text-gray-300 ml-1.5 truncate">
            {activeFile.name}.md
          </span>
        </div>

        {/* 文档内容 */}
        <div className="flex-1 overflow-auto p-4">
          <div className="prose prose-invert prose-sm max-w-none">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={markdownComponents}
            >
              {activeDocContent}
            </ReactMarkdown>
          </div>
        </div>
      </div>
    )
  }

  // 无文档时的占位
  return (
    <div className="h-full flex items-center justify-center bg-[#1e1e1e] text-gray-500">
      <div className="text-center">
        <svg
          className="w-12 h-12 mx-auto mb-3 text-gray-600"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <p className="text-sm">暂无对应文档</p>
        <p className="text-xs text-gray-600 mt-1">选择源码文件以查看文档</p>
      </div>
    </div>
  )
}

// Markdown 渲染自定义组件
const markdownComponents = {
  code({ className, children }: { className?: string; children?: React.ReactNode }) {
    return (
      <code className={`${className || ''} bg-[#1e1e1e] px-1 py-0.5 rounded text-[#ce9178]`}>
        {children}
      </code>
    )
  },
  pre({ children }: { children?: React.ReactNode }) {
    return (
      <pre className="bg-[#0d0d0d] p-3 rounded overflow-x-auto border border-[#3c3c3c]">
        {children}
      </pre>
    )
  },
  h1({ children }: { children?: React.ReactNode }) {
    return <h1 className="text-lg font-bold text-gray-100 border-b border-[#3c3c3c] pb-2">{children}</h1>
  },
  h2({ children }: { children?: React.ReactNode }) {
    return <h2 className="text-base font-bold text-gray-200 mt-4">{children}</h2>
  },
  h3({ children }: { children?: React.ReactNode }) {
    return <h3 className="text-sm font-bold text-gray-300 mt-3">{children}</h3>
  },
  table({ children }: { children?: React.ReactNode }) {
    return (
      <div className="overflow-x-auto">
        <table className="border-collapse border border-[#3c3c3c] w-full text-sm">
          {children}
        </table>
      </div>
    )
  },
  th({ children }: { children?: React.ReactNode }) {
    return (
      <th className="border border-[#3c3c3c] bg-[#2d2d2d] px-3 py-1.5 text-left text-gray-300">
        {children}
      </th>
    )
  },
  td({ children }: { children?: React.ReactNode }) {
    return (
      <td className="border border-[#3c3c3c] px-3 py-1.5 text-gray-400">
        {children}
      </td>
    )
  }
}

function DocHeaderIcon(): JSX.Element {
  return (
    <svg className="w-4 h-4 text-blue-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
      />
    </svg>
  )
}
