import { useState, useEffect } from 'react'
import { Allotment } from 'allotment'
import 'allotment/dist/style.css'
import { SplitFileTree } from '../file-tree/SplitFileTree'
import { CodeEditor } from '../editor/CodeEditor'
import { ChatPanel } from '../chat/ChatPanel'
import { KnowledgeGraph } from '../graph/KnowledgeGraph'
import { DocViewer } from '../editor/DocViewer'
import { Toolbar } from './Toolbar'
import { useFileStore, ViewMode } from '../../stores/fileStore'
import { useEditorStore } from '../../stores/editorStore'
import { useDocStore } from '../../stores/docStore'

type CenterTab = 'code' | 'graph'

export function MainLayout(): JSX.Element {
  const [centerTab, setCenterTab] = useState<CenterTab>('code')
  const [rightPanelVisible, setRightPanelVisible] = useState(true)
  const { viewMode, setViewMode, projectPath } = useFileStore()
  const {
    activeDocContent,
    selectedDirSummary,
    setDocsBasePath,
    setProjectPath: setEditorProjectPath
  } = useEditorStore()
  const { status: docGenStatus, docsPath } = useDocStore()

  // 同步 projectPath 到 editorStore
  useEffect(() => {
    setEditorProjectPath(projectPath)
  }, [projectPath, setEditorProjectPath])

  // 当文档生成开始时，设置 docsBasePath（这样生成过程中也能查看已生成的文档）
  useEffect(() => {
    if (docsPath) {
      setDocsBasePath(docsPath)
    }
  }, [docsPath, setDocsBasePath])

  // 当文档生成完成时，自动启用分屏模式
  useEffect(() => {
    if (docGenStatus === 'completed' && docsPath) {
      setViewMode('code-and-docs')
    }
  }, [docGenStatus, docsPath, setViewMode])

  // 检查是否有文档可显示
  // 允许在生成过程中查看已生成的文档
  const hasDocContent = activeDocContent !== null || selectedDirSummary !== null
  const canShowDocs = docsPath !== null && (docGenStatus === 'running' || docGenStatus === 'completed' || docGenStatus === 'cancelled')

  return (
    <div className="h-screen w-screen flex flex-col bg-[#1e1e1e]">
      <Toolbar />
      <div className="flex-1 overflow-hidden relative">
        <Allotment>
          {/* 左侧边栏 - 分割文件树 */}
          <Allotment.Pane preferredSize={280} minSize={200} maxSize={400}>
            <div className="h-full bg-[#252526] border-r border-[#3c3c3c]">
              <SplitFileTree />
            </div>
          </Allotment.Pane>

          {/* 中间面板 */}
          <Allotment.Pane minSize={300}>
            <div className="h-full bg-[#1e1e1e] flex flex-col">
              {/* 顶部标签栏 */}
              <div className="h-9 flex items-center bg-[#252526] border-b border-[#3c3c3c] px-2 shrink-0">
                {/* 左侧：视图切换标签 */}
                <div className="flex items-center">
                  <button
                    onClick={() => setCenterTab('code')}
                    className={`px-3 py-1 text-xs rounded-t transition-colors ${
                      centerTab === 'code'
                        ? 'bg-[#1e1e1e] text-white border-t border-x border-[#3c3c3c] -mb-px'
                        : 'text-gray-400 hover:text-gray-200'
                    }`}
                  >
                    <span className="flex items-center gap-1.5">
                      <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                      </svg>
                      Code
                    </span>
                  </button>
                  <button
                    onClick={() => setCenterTab('graph')}
                    className={`px-3 py-1 text-xs rounded-t transition-colors ml-1 ${
                      centerTab === 'graph'
                        ? 'bg-[#1e1e1e] text-white border-t border-x border-[#3c3c3c] -mb-px'
                        : 'text-gray-400 hover:text-gray-200'
                    }`}
                  >
                    <span className="flex items-center gap-1.5">
                      <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
                      </svg>
                      Graph
                    </span>
                  </button>
                </div>

                {/* 右侧：视图模式切换（仅在 code 标签且有文档时显示） */}
                {centerTab === 'code' && canShowDocs && (
                  <div className="ml-auto flex items-center gap-1">
                    <span className="text-xs text-gray-500 mr-2">View:</span>
                    <button
                      onClick={() => setViewMode('code-only')}
                      className={`px-2 py-0.5 text-xs rounded transition-colors ${
                        viewMode === 'code-only'
                          ? 'bg-[#0e639c] text-white'
                          : 'text-gray-400 hover:text-white hover:bg-[#3c3c3c]'
                      }`}
                      title="Only show source code"
                    >
                      Code
                    </button>
                    <button
                      onClick={() => setViewMode('code-and-docs')}
                      className={`px-2 py-0.5 text-xs rounded transition-colors ${
                        viewMode === 'code-and-docs'
                          ? 'bg-[#0e639c] text-white'
                          : 'text-gray-400 hover:text-white hover:bg-[#3c3c3c]'
                      }`}
                      title="Show source code and documentation side by side"
                    >
                      Code + Docs
                    </button>
                  </div>
                )}
              </div>

              {/* 内容区域 */}
              <div className="flex-1 overflow-hidden">
                {centerTab === 'code' && (
                  <CodeEditorWithDocs viewMode={viewMode} hasDocContent={hasDocContent} canShowDocs={canShowDocs} />
                )}
                {centerTab === 'graph' && <KnowledgeGraph />}
              </div>
            </div>
          </Allotment.Pane>

          {/* 右侧边栏 - AI Chat */}
          <Allotment.Pane
            preferredSize={380}
            minSize={300}
            maxSize={600}
            visible={rightPanelVisible}
          >
            <div className="h-full bg-[#252526] border-l border-[#3c3c3c]">
              <ChatPanel />
            </div>
          </Allotment.Pane>
        </Allotment>

        {/* 右侧边栏折叠/展开按钮 */}
        <button
          onClick={() => setRightPanelVisible(!rightPanelVisible)}
          className={`absolute top-1/2 -translate-y-1/2 z-10 w-5 h-12 flex items-center justify-center
            bg-[#3c3c3c] hover:bg-[#4c4c4c] text-gray-400 hover:text-white
            transition-all duration-200 rounded-l
            ${rightPanelVisible ? 'right-0' : 'right-0'}`}
          title={rightPanelVisible ? 'Hide AI Assistant' : 'Show AI Assistant'}
        >
          <svg
            className={`w-3 h-3 transition-transform duration-200 ${rightPanelVisible ? '' : 'rotate-180'}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>
    </div>
  )
}

/**
 * 代码编辑器组件（支持分屏显示文档）
 */
function CodeEditorWithDocs({
  viewMode,
  hasDocContent,
  canShowDocs
}: {
  viewMode: ViewMode
  hasDocContent: boolean
  canShowDocs: boolean
}): JSX.Element {
  // 如果是仅代码模式，或者没有文档功能，只显示代码编辑器
  if (viewMode === 'code-only' || !canShowDocs) {
    return <CodeEditor />
  }

  // 分屏模式：左侧代码，右侧文档
  return (
    <Allotment>
      <Allotment.Pane minSize={300}>
        <CodeEditor />
      </Allotment.Pane>
      <Allotment.Pane minSize={250} preferredSize={400}>
        <DocViewer />
      </Allotment.Pane>
    </Allotment>
  )
}
