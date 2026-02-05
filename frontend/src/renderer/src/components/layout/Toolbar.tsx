import { useState, useEffect, useRef } from 'react'
import { useFileStore } from '../../stores/fileStore'
import { useChatStore } from '../../stores/chatStore'
import { useDocStore } from '../../stores/docStore'
import { SettingsModal } from '../settings/SettingsModal'

export function Toolbar(): JSX.Element {
  const {
    projectPath, loadProject, isLoading,
    recentProjects, loadRecentProjects, openProjectByPath, removeRecentProject
  } = useFileStore()
  const { wsStatus, apiConfigured } = useChatStore()
  const { status: docStatus, startGeneration, cancelGeneration, reset: resetDocGen, initPort } = useDocStore()
  const [showSettings, setShowSettings] = useState(false)
  const [showRecent, setShowRecent] = useState(false)
  const recentMenuRef = useRef<HTMLDivElement>(null)
  const hasAutoOpenedRef = useRef(false)

  // 初始化后端端口
  useEffect(() => {
    window.api.getBackendPort().then(initPort)
  }, [initPort])

  // 加载最近项目列表
  useEffect(() => {
    loadRecentProjects()
  }, [loadRecentProjects])

  // 启动时自动打开上次的项目
  useEffect(() => {
    if (hasAutoOpenedRef.current) return
    if (recentProjects.length === 0) return
    if (projectPath) return // 已经有打开的项目
    if (isLoading) return

    hasAutoOpenedRef.current = true
    const lastProject = recentProjects[0]
    console.log('[Toolbar] Auto-opening last project:', lastProject.path)
    openProjectByPath(lastProject.path)
  }, [recentProjects, projectPath, isLoading, openProjectByPath])

  // 点击外部关闭下拉菜单
  useEffect(() => {
    function handleClickOutside(event: MouseEvent): void {
      if (recentMenuRef.current && !recentMenuRef.current.contains(event.target as Node)) {
        setShowRecent(false)
      }
    }
    if (showRecent) {
      document.addEventListener('mousedown', handleClickOutside)
    }
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [showRecent])

  const projectName = projectPath ? projectPath.split(/[\\/]/).pop() : null

  // Connection status: green only when WebSocket connected AND API configured
  const isFullyConnected = wsStatus === 'connected' && apiConfigured
  const statusColor = isFullyConnected
    ? 'bg-green-500'
    : wsStatus === 'connecting'
    ? 'bg-yellow-500'
    : wsStatus === 'connected' && !apiConfigured
    ? 'bg-orange-500'
    : 'bg-red-500'

  const statusText = isFullyConnected
    ? 'AI Ready'
    : wsStatus === 'connecting'
    ? 'Connecting...'
    : wsStatus === 'connected' && !apiConfigured
    ? 'API Key Required'
    : 'Disconnected'

  const isDocGenerating = docStatus === 'running'

  const handleGenerateDocs = async () => {
    if (!projectPath) return
    await startGeneration(projectPath)
  }

  const handleCancelGeneration = () => {
    cancelGeneration()
  }

  const handleOpenRecent = async (path: string) => {
    setShowRecent(false)
    await openProjectByPath(path)
  }

  const handleRemoveRecent = async (e: React.MouseEvent, path: string) => {
    e.stopPropagation()
    await removeRecentProject(path)
  }

  // 格式化时间显示
  const formatTime = (timestamp: number): string => {
    const date = new Date(timestamp)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)
    const diffHours = Math.floor(diffMs / 3600000)
    const diffDays = Math.floor(diffMs / 86400000)

    if (diffMins < 1) return 'Just now'
    if (diffMins < 60) return `${diffMins}m ago`
    if (diffHours < 24) return `${diffHours}h ago`
    if (diffDays < 7) return `${diffDays}d ago`
    return date.toLocaleDateString()
  }

  return (
    <>
      <div className="h-10 bg-[#3c3c3c] border-b border-[#2d2d2d] flex items-center px-4 justify-between">
        <div className="flex items-center gap-3">
          {/* 打开项目按钮 + 最近项目下拉 */}
          <div className="relative" ref={recentMenuRef}>
            <div className="flex">
              <button
                onClick={loadProject}
                disabled={isLoading}
                className="h-[26px] px-3 text-sm bg-[#0e639c] hover:bg-[#1177bb] text-white rounded-l disabled:opacity-50"
              >
                {isLoading ? 'Loading...' : 'Open Project'}
              </button>
              <button
                onClick={() => setShowRecent(!showRecent)}
                disabled={isLoading}
                className="h-[26px] px-1.5 bg-[#0e639c] hover:bg-[#1177bb] text-white rounded-r border-l border-[#0a4f7d] disabled:opacity-50 flex items-center"
                title="Recent Projects"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
            </div>

            {/* 最近项目下拉菜单 */}
            {showRecent && (
              <div className="absolute top-full left-0 mt-1 w-80 bg-[#252526] border border-[#454545] rounded shadow-lg z-50 max-h-80 overflow-y-auto">
                {recentProjects.length === 0 ? (
                  <div className="px-3 py-4 text-sm text-gray-500 text-center">
                    No recent projects
                  </div>
                ) : (
                  recentProjects.map((project) => (
                    <div
                      key={project.path}
                      onClick={() => handleOpenRecent(project.path)}
                      className={`px-3 py-2 hover:bg-[#2a2d2e] cursor-pointer flex items-center justify-between group ${
                        project.path === projectPath ? 'bg-[#37373d]' : ''
                      }`}
                    >
                      <div className="flex-1 min-w-0 mr-2">
                        <div className="text-sm text-gray-200 truncate">{project.name}</div>
                        <div className="text-xs text-gray-500 truncate">{project.path}</div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        <span className="text-xs text-gray-500">{formatTime(project.lastOpened)}</span>
                        <button
                          onClick={(e) => handleRemoveRecent(e, project.path)}
                          className="p-0.5 hover:bg-[#505050] rounded opacity-0 group-hover:opacity-100 transition-opacity"
                          title="Remove from recent"
                        >
                          <svg className="w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                          </svg>
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}
          </div>

          {projectPath && (
            <>
              {!isDocGenerating ? (
                <button
                  onClick={handleGenerateDocs}
                  disabled={!isFullyConnected}
                  className="px-3 py-1 text-sm bg-[#2ea043] hover:bg-[#3fb950] text-white rounded disabled:opacity-50 flex items-center gap-1.5"
                  title={!isFullyConnected ? 'Configure API key first' : 'Generate documentation for this project'}
                >
                  <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  Generate Docs
                </button>
              ) : (
                <button
                  onClick={handleCancelGeneration}
                  className="px-3 py-1 text-sm bg-red-600 hover:bg-red-700 text-white rounded flex items-center gap-1.5"
                >
                  <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                  Cancel
                </button>
              )}
            </>
          )}

          {projectName && (
            <span className="text-sm text-gray-300">
              <span className="text-white font-medium">{projectName}</span>
            </span>
          )}
        </div>

        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <div className={`w-2 h-2 rounded-full ${statusColor}`} />
            <span className="text-xs text-gray-400">{statusText}</span>
          </div>

          <button
            onClick={() => setShowSettings(true)}
            className="p-1.5 hover:bg-[#505050] rounded"
            title="Settings"
          >
            <svg className="w-4 h-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>

      {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
    </>
  )
}
