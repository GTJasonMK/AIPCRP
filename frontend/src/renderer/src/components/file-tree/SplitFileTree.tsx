import { useEffect } from 'react'
import { Allotment } from 'allotment'
import { useFileStore, FileTreeNode } from '../../stores/fileStore'
import { useEditorStore } from '../../stores/editorStore'
import { useDocStore, DocTreeNode, DocFileStatus } from '../../stores/docStore'

/**
 * 分割文件树组件
 *
 * 上部分显示源代码目录（过滤掉文档目录）
 * 下部分显示文档目录（带生成进度指示）
 */
export function SplitFileTree(): JSX.Element {
  const {
    projectPath,
    fileTree,
    expandedPaths,
    toggleExpanded,
    docsExpandedPaths,
    toggleDocsExpanded,
    expandAllDocsPaths,
    setViewMode,
    docsPath: existingDocsPath  // 从 fileStore 获取（项目加载时检测到的已有文档目录）
  } = useFileStore()

  const { openFile, loadDirSummary, setDocsBasePath } = useEditorStore()
  const {
    docsTree,
    docsPath,
    status: docGenStatus,
    generateDocsTreeFromSource,
    loadExistingDocsTree,
    reset: resetDocStore,
    stats
  } = useDocStore()

  // 当项目路径变化时，重置 docStore 状态
  useEffect(() => {
    if (projectPath) {
      console.log('[SplitFileTree] Project changed, resetting docStore')
      resetDocStore()
    }
  }, [projectPath, resetDocStore])

  // 检测已存在的文档目录并自动加载
  useEffect(() => {
    if (existingDocsPath && projectPath && fileTree.length > 0 && docGenStatus === 'idle' && docsTree.length === 0) {
      console.log('[SplitFileTree] Loading existing docs tree:', existingDocsPath)
      // 获取 .docs 目录的实际文件列表，用于判断哪些文档已生成
      window.api.getFileTree(existingDocsPath).then((docsFileTree) => {
        const existingPaths = new Set<string>()
        const collectPaths = (nodes: { name: string; path: string; type: string; children?: typeof nodes }[]): void => {
          for (const node of nodes) {
            // 规范化路径
            existingPaths.add(node.path.replace(/\\/g, '/'))
            if (node.children) collectPaths(node.children)
          }
        }
        collectPaths(docsFileTree)
        console.log('[SplitFileTree] Found existing doc files:', existingPaths.size)
        loadExistingDocsTree(fileTree, existingDocsPath, projectPath, existingPaths)
      }).catch((err) => {
        console.error('[SplitFileTree] Failed to read docs directory:', err)
      })
    }
  }, [existingDocsPath, projectPath, fileTree, docGenStatus, docsTree.length, loadExistingDocsTree])

  // 后备方案：如果 startGeneration 时 fileTree/projectPath 还没准备好，
  // 则在 useEffect 中生成文档树（只在首次生成，避免重复生成导致状态重置）
  useEffect(() => {
    if (docGenStatus === 'running' && docsPath && projectPath && fileTree.length > 0 && docsTree.length === 0) {
      console.log('[SplitFileTree] Fallback: generateDocsTreeFromSource in useEffect')
      generateDocsTreeFromSource(fileTree, docsPath, projectPath)
    }
  }, [docGenStatus, docsPath, projectPath, fileTree, docsTree.length, generateDocsTreeFromSource])

  // 文档树生成后自动展开所有目录，方便查看生成进度
  useEffect(() => {
    if (docsTree.length > 0 && docsExpandedPaths.size === 0) {
      const allDirPaths = collectDirPaths(docsTree)
      if (allDirPaths.length > 0) {
        expandAllDocsPaths(allDirPaths)
      }
    }
  }, [docsTree, docsExpandedPaths.size, expandAllDocsPaths])

  // 当 docsPath 有值时设置 docsBasePath（这样生成过程中也能查看已生成的文档）
  useEffect(() => {
    if (docsPath) {
      setDocsBasePath(docsPath)
    }
  }, [docsPath, setDocsBasePath])

  if (!projectPath) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-gray-500 text-sm text-center">
          Click "Open Project" button above
          <br />
          to select a code directory
        </p>
      </div>
    )
  }

  const hasDocsTree = docsTree.length > 0
  const isGenerating = docGenStatus === 'running'
  const isCompleted = docGenStatus === 'completed'

  // 处理源码文件点击
  const handleSourceFileClick = (path: string) => {
    openFile(path)
  }

  // 处理源码目录点击 - 展开/折叠，同时尝试加载目录总结
  const handleSourceDirClick = (path: string, name: string) => {
    toggleExpanded(path)
    if (isCompleted) {
      loadDirSummary(path, name)
    }
  }

  // 处理文档文件点击 - 打开对应的源码文件并切换到分屏模式
  const handleDocFileClick = (sourcePath: string) => {
    openFile(sourcePath)
    // 自动切换到分屏模式以显示文档
    setViewMode('code-and-docs')
  }

  return (
    <div className="h-full flex flex-col">
      <Allotment vertical>
        {/* 上部分：源代码目录 */}
        <Allotment.Pane minSize={100}>
          <div className="h-full flex flex-col">
            <div className="px-3 py-2 border-b border-[#3c3c3c] flex items-center justify-between">
              <span className="text-xs text-gray-400 uppercase tracking-wider">Source</span>
              <SourceIcon />
            </div>
            <div className="flex-1 overflow-auto py-1">
              {fileTree.map(node => (
                <SourceTreeItem
                  key={node.path}
                  node={node}
                  depth={0}
                  expandedPaths={expandedPaths}
                  onFileClick={handleSourceFileClick}
                  onDirClick={handleSourceDirClick}
                />
              ))}
            </div>
          </div>
        </Allotment.Pane>

        {/* 下部分：文档目录（显示生成进度） */}
        {(hasDocsTree || isGenerating) && (
          <Allotment.Pane minSize={100} preferredSize={250}>
            <div className="h-full flex flex-col border-t border-[#3c3c3c]">
              <div className="px-3 py-2 border-b border-[#3c3c3c] flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-400 uppercase tracking-wider">Docs</span>
                  {isGenerating && (
                    <span className="text-xs text-blue-400">
                      ({stats?.processed_files || 0}/{stats?.total_files || 0})
                    </span>
                  )}
                  {isCompleted && (
                    <span className="text-xs text-green-400">Complete</span>
                  )}
                </div>
                <DocsIcon />
              </div>
              <div className="flex-1 overflow-auto py-1">
                {docsTree.map(node => (
                  <DocTreeItem
                    key={node.path}
                    node={node}
                    depth={0}
                    expandedPaths={docsExpandedPaths}
                    onToggle={toggleDocsExpanded}
                    onFileClick={handleDocFileClick}
                    isGenerating={isGenerating}
                  />
                ))}
              </div>
            </div>
          </Allotment.Pane>
        )}
      </Allotment>
    </div>
  )
}

interface SourceTreeItemProps {
  node: FileTreeNode
  depth: number
  expandedPaths: Set<string>
  onFileClick: (path: string) => void
  onDirClick: (path: string, name: string) => void
}

function SourceTreeItem({
  node,
  depth,
  expandedPaths,
  onFileClick,
  onDirClick
}: SourceTreeItemProps): JSX.Element {
  const isExpanded = expandedPaths.has(node.path)
  const isDirectory = node.type === 'directory'
  const paddingLeft = 12 + depth * 16

  const handleClick = () => {
    if (isDirectory) {
      onDirClick(node.path, node.name)
    } else {
      onFileClick(node.path)
    }
  }

  const icon = isDirectory
    ? isExpanded
      ? <ChevronDownIcon />
      : <ChevronRightIcon />
    : <FileIcon extension={node.name.split('.').pop() || ''} />

  return (
    <div>
      <div
        className="flex items-center py-0.5 px-2 hover:bg-[#2a2d2e] cursor-pointer"
        style={{ paddingLeft }}
        onClick={handleClick}
      >
        <span className="w-4 h-4 flex items-center justify-center mr-1">
          {icon}
        </span>
        <span className="text-sm truncate">{node.name}</span>
      </div>

      {isDirectory && isExpanded && node.children && (
        <div>
          {node.children.map(child => (
            <SourceTreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              expandedPaths={expandedPaths}
              onFileClick={onFileClick}
              onDirClick={onDirClick}
            />
          ))}
        </div>
      )}
    </div>
  )
}

interface DocTreeItemProps {
  node: DocTreeNode
  depth: number
  expandedPaths: Set<string>
  onToggle: (path: string) => void
  onFileClick: (sourcePath: string) => void
  isGenerating: boolean
}

function DocTreeItem({
  node,
  depth,
  expandedPaths,
  onToggle,
  onFileClick,
  isGenerating
}: DocTreeItemProps): JSX.Element {
  const isExpanded = expandedPaths.has(node.sourcePath)
  const isDirectory = node.type === 'directory'
  const paddingLeft = 12 + depth * 16

  // 文件是否可点击（只有已完成的文档才能点击查看）
  const isFileClickable = !isDirectory && node.status === 'completed'

  const handleClick = () => {
    if (isDirectory) {
      onToggle(node.sourcePath)
    } else if (isFileClickable) {
      // 点击已完成的文档文件，打开对应的源码文件（文档会自动加载）
      onFileClick(node.sourcePath)
    }
  }

  // 状态样式
  const statusStyles = getStatusStyles(node.status, isGenerating)

  const icon = isDirectory
    ? isExpanded
      ? <ChevronDownIcon />
      : <ChevronRightIcon />
    : <DocFileIcon status={node.status} />

  return (
    <div>
      <div
        className={`flex items-center py-0.5 px-2 ${statusStyles.bg} ${isDirectory || isFileClickable ? 'cursor-pointer' : 'cursor-default'}`}
        style={{ paddingLeft }}
        onClick={handleClick}
      >
        <span className="w-4 h-4 flex items-center justify-center mr-1">
          {icon}
        </span>
        <span className={`text-sm truncate ${statusStyles.text}`}>
          {node.name}
        </span>
        {node.status === 'processing' && (
          <span className="ml-2">
            <LoadingSpinner />
          </span>
        )}
        {node.status === 'completed' && (
          <span className="ml-2">
            <CheckIcon />
          </span>
        )}
        {node.status === 'failed' && (
          <span className="ml-2">
            <ErrorIcon />
          </span>
        )}
        {node.status === 'interrupted' && (
          <span className="ml-2">
            <InterruptedIcon />
          </span>
        )}
      </div>

      {isDirectory && isExpanded && node.children && (
        <div>
          {node.children.map(child => (
            <DocTreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              expandedPaths={expandedPaths}
              onToggle={onToggle}
              onFileClick={onFileClick}
              isGenerating={isGenerating}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function getStatusStyles(status: DocFileStatus, isGenerating: boolean): { bg: string; text: string } {
  switch (status) {
    case 'processing':
      return { bg: 'bg-blue-500/10', text: 'text-blue-300' }
    case 'completed':
      return { bg: 'hover:bg-[#2a2d2e]', text: 'text-green-400' }
    case 'failed':
      return { bg: 'hover:bg-[#2a2d2e]', text: 'text-red-400' }
    case 'skipped':
      return { bg: 'hover:bg-[#2a2d2e]', text: 'text-yellow-500' }
    case 'interrupted':
      return { bg: 'bg-orange-500/10', text: 'text-orange-400' }
    case 'pending':
    default:
      return {
        bg: isGenerating ? '' : 'hover:bg-[#2a2d2e]',
        text: isGenerating ? 'text-gray-500' : 'text-gray-400'
      }
  }
}

// Icons
function ChevronRightIcon(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
    </svg>
  )
}

function ChevronDownIcon(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
    </svg>
  )
}

function FileIcon({ extension }: { extension: string }): JSX.Element {
  const colors: Record<string, string> = {
    ts: '#3178c6',
    tsx: '#3178c6',
    js: '#f7df1e',
    jsx: '#f7df1e',
    py: '#3776ab',
    json: '#cbcb41',
    md: '#083fa1',
    html: '#e34c26',
    css: '#264de4',
    scss: '#c6538c',
    vue: '#41b883',
    go: '#00add8',
    rs: '#dea584',
    java: '#b07219',
    cpp: '#f34b7d',
    c: '#555555'
  }

  const color = colors[extension] || '#d4d4d4'

  return (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none">
      <path
        d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"
        stroke={color}
        strokeWidth="1.5"
        fill="none"
      />
      <path d="M14 2v6h6" stroke={color} strokeWidth="1.5" fill="none" />
    </svg>
  )
}

function DocFileIcon({ status }: { status: DocFileStatus }): JSX.Element {
  let color = '#083fa1'
  if (status === 'completed') color = '#3fb950'
  else if (status === 'processing') color = '#58a6ff'
  else if (status === 'failed') color = '#f85149'
  else if (status === 'interrupted') color = '#f97316'  // orange
  else if (status === 'pending') color = '#6e7681'

  return (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none">
      <path
        d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"
        stroke={color}
        strokeWidth="1.5"
        fill="none"
      />
      <path d="M14 2v6h6" stroke={color} strokeWidth="1.5" fill="none" />
      <path d="M8 13h8M8 17h5" stroke={color} strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  )
}

function SourceIcon(): JSX.Element {
  return (
    <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
    </svg>
  )
}

function DocsIcon(): JSX.Element {
  return (
    <svg className="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
    </svg>
  )
}

function LoadingSpinner(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-blue-400 animate-spin" fill="none" viewBox="0 0 24 24">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  )
}

function CheckIcon(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
    </svg>
  )
}

function ErrorIcon(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  )
}

function InterruptedIcon(): JSX.Element {
  return (
    <svg className="w-3 h-3 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
    </svg>
  )
}

// 递归收集文档树中所有目录节点的 sourcePath
function collectDirPaths(nodes: DocTreeNode[]): string[] {
  const paths: string[] = []
  for (const node of nodes) {
    if (node.type === 'directory') {
      paths.push(node.sourcePath)
      if (node.children) {
        paths.push(...collectDirPaths(node.children))
      }
    }
  }
  return paths
}
