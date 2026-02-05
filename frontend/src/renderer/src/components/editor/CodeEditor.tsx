import Editor, { OnMount } from '@monaco-editor/react'
import { useEditorStore } from '../../stores/editorStore'

// 文件扩展名到Monaco语言的映射
const languageMap: Record<string, string> = {
  ts: 'typescript',
  tsx: 'typescript',
  js: 'javascript',
  jsx: 'javascript',
  py: 'python',
  json: 'json',
  md: 'markdown',
  html: 'html',
  css: 'css',
  scss: 'scss',
  less: 'less',
  java: 'java',
  go: 'go',
  rs: 'rust',
  cpp: 'cpp',
  c: 'c',
  h: 'c',
  hpp: 'cpp',
  vue: 'html',
  xml: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
  sql: 'sql',
  sh: 'shell',
  bash: 'shell',
  dockerfile: 'dockerfile'
}

export function CodeEditor(): JSX.Element {
  const { openFiles, activeFile, closeFile, setActiveFile, setSelectedCode } = useEditorStore()

  const handleEditorMount: OnMount = (editor) => {
    // 监听选择变化
    editor.onDidChangeCursorSelection((e) => {
      const selection = editor.getModel()?.getValueInRange(e.selection)
      setSelectedCode(selection || '')
    })

    // 添加右键菜单项
    editor.addAction({
      id: 'ask-ai',
      label: '询问AI关于选中代码',
      contextMenuGroupId: 'navigation',
      keybindings: [],
      run: () => {
        const selection = editor.getModel()?.getValueInRange(editor.getSelection()!)
        if (selection) {
          window.dispatchEvent(new CustomEvent('ask-ai', { detail: selection }))
        }
      }
    })
  }

  if (!activeFile) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        <div className="text-center">
          <svg className="w-16 h-16 mx-auto mb-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <p className="text-sm">选择一个文件开始查看代码</p>
        </div>
      </div>
    )
  }

  const language = languageMap[activeFile.extension] || 'plaintext'

  return (
    <div className="h-full flex flex-col">
      {/* 标签栏 */}
      <div className="h-9 bg-[#252526] flex items-center border-b border-[#3c3c3c] overflow-x-auto">
        {openFiles.map(file => (
          <div
            key={file.path}
            className={`flex items-center gap-1 px-3 h-full border-r border-[#3c3c3c] cursor-pointer ${
              file.path === activeFile.path ? 'bg-[#1e1e1e]' : 'bg-[#2d2d2d] hover:bg-[#3c3c3c]'
            }`}
            onClick={() => setActiveFile(file.path)}
          >
            <span className="text-sm truncate max-w-[150px]">{file.name}</span>
            <button
              className="ml-1 p-0.5 hover:bg-[#505050] rounded opacity-60 hover:opacity-100"
              onClick={(e) => {
                e.stopPropagation()
                closeFile(file.path)
              }}
            >
              <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        ))}
      </div>

      {/* 编辑器 */}
      <div className="flex-1">
        <Editor
          height="100%"
          theme="vs-dark"
          path={activeFile.path}
          defaultLanguage={language}
          defaultValue={activeFile.content}
          options={{
            readOnly: true,
            automaticLayout: true,
            scrollBeyondLastLine: false,
            minimap: { enabled: true },
            fontSize: 14,
            lineNumbers: 'on',
            folding: true,
            wordWrap: 'off',
            renderLineHighlight: 'all',
            scrollbar: {
              verticalScrollbarSize: 10,
              horizontalScrollbarSize: 10
            }
          }}
          onMount={handleEditorMount}
        />
      </div>
    </div>
  )
}
