import { useState, useEffect, useRef } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { useChatStore } from '../../stores/chatStore'
import { useFileStore } from '../../stores/fileStore'
import { useEditorStore } from '../../stores/editorStore'

// 生成文件树摘要
function generateFileTreeSummary(tree: any[], depth = 0, maxDepth = 2): string {
  if (depth > maxDepth) return ''
  let summary = ''
  for (const node of tree.slice(0, 15)) {
    const indent = '  '.repeat(depth)
    summary += `${indent}${node.name}\n`
    if (node.children) {
      summary += generateFileTreeSummary(node.children, depth + 1, maxDepth)
    }
  }
  return summary
}

export function ChatPanel(): JSX.Element {
  const { messages, isStreaming, wsStatus, sendMessage, clearMessages } = useChatStore()
  const { projectPath, fileTree } = useFileStore()
  const { activeFile, selectedCode } = useEditorStore()
  const [input, setInput] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)

  // 监听"询问AI"事件
  useEffect(() => {
    const handler = (e: CustomEvent) => {
      const code = e.detail as string
      setInput(`请解释以下代码:\n\`\`\`\n${code}\n\`\`\``)
    }
    window.addEventListener('ask-ai', handler as EventListener)
    return () => window.removeEventListener('ask-ai', handler as EventListener)
  }, [])

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const handleSend = () => {
    if (!input.trim() || isStreaming) return

    const context = {
      projectPath: projectPath || undefined,
      currentFile: activeFile?.path,
      currentFileContent: activeFile?.content,
      selectedCode: selectedCode || undefined,
      fileTreeSummary: generateFileTreeSummary(fileTree)
    }

    sendMessage(input, context)
    setInput('')
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const suggestedQuestions = [
    '这个项目的整体架构是怎样的?',
    '项目使用了哪些主要技术和框架?',
    '有哪些可以改进的地方?',
    '请解释当前文件的作用'
  ]

  return (
    <div className="h-full flex flex-col">
      {/* 头部 */}
      <div className="px-4 py-3 border-b border-[#3c3c3c] flex items-center justify-between">
        <h2 className="text-white font-medium">AI 代码助手</h2>
        {messages.length > 0 && (
          <button
            onClick={clearMessages}
            className="text-xs text-gray-400 hover:text-white"
          >
            清空对话
          </button>
        )}
      </div>

      {/* 消息列表 */}
      <div className="flex-1 overflow-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="space-y-4">
            <p className="text-gray-400 text-sm">
              我是AI代码助手，可以帮助你理解和分析代码。
              {wsStatus !== 'connected' && (
                <span className="text-yellow-500 block mt-2">
                  正在连接AI服务...
                </span>
              )}
            </p>
            <div className="space-y-2">
              <p className="text-xs text-gray-500">试试这些问题:</p>
              {suggestedQuestions.map((q, i) => (
                <button
                  key={i}
                  onClick={() => setInput(q)}
                  className="block w-full text-left text-sm text-blue-400 hover:text-blue-300 py-1"
                >
                  {q}
                </button>
              ))}
            </div>
          </div>
        ) : (
          messages.map(msg => (
            <div
              key={msg.id}
              className={`${msg.role === 'user' ? 'ml-8' : 'mr-8'}`}
            >
              <div
                className={`rounded-lg p-3 ${
                  msg.role === 'user'
                    ? 'bg-[#0e639c] text-white'
                    : 'bg-[#3c3c3c] text-gray-200'
                }`}
              >
                {msg.role === 'assistant' ? (
                  <div className="prose prose-invert prose-sm max-w-none">
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      components={{
                        code({ className, children }) {
                          return (
                            <code className={`${className} bg-[#1e1e1e] px-1 py-0.5 rounded`}>
                              {children}
                            </code>
                          )
                        },
                        pre({ children }) {
                          return (
                            <pre className="bg-[#1e1e1e] p-3 rounded overflow-x-auto">
                              {children}
                            </pre>
                          )
                        }
                      }}
                    >
                      {msg.content || (isStreaming ? '...' : '')}
                    </ReactMarkdown>
                  </div>
                ) : (
                  <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                )}
              </div>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* 输入框 */}
      <div className="p-4 border-t border-[#3c3c3c]">
        <div className="flex gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="输入问题... (Enter发送, Shift+Enter换行)"
            className="flex-1 bg-[#3c3c3c] text-white text-sm rounded px-3 py-2 resize-none focus:outline-none focus:ring-1 focus:ring-[#0e639c]"
            rows={3}
            disabled={wsStatus !== 'connected'}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || isStreaming || wsStatus !== 'connected'}
            className="px-4 bg-[#0e639c] hover:bg-[#1177bb] text-white rounded disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isStreaming ? (
              <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
