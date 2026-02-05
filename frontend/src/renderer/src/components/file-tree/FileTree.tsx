import { useFileStore } from '../../stores/fileStore'
import { useEditorStore } from '../../stores/editorStore'

interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

export function FileTree(): JSX.Element {
  const { projectPath, fileTree, expandedPaths, toggleExpanded } = useFileStore()
  const { openFile } = useEditorStore()

  if (!projectPath) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-gray-500 text-sm text-center">
          点击上方"打开项目"按钮
          <br />
          选择一个代码目录
        </p>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <div className="px-3 py-2 border-b border-[#3c3c3c]">
        <span className="text-xs text-gray-400 uppercase tracking-wider">资源管理器</span>
      </div>
      <div className="flex-1 overflow-auto py-1">
        {fileTree.map(node => (
          <FileTreeItem
            key={node.path}
            node={node}
            depth={0}
            expandedPaths={expandedPaths}
            onToggle={toggleExpanded}
            onFileClick={openFile}
          />
        ))}
      </div>
    </div>
  )
}

interface FileTreeItemProps {
  node: FileTreeNode
  depth: number
  expandedPaths: Set<string>
  onToggle: (path: string) => void
  onFileClick: (path: string) => void
}

function FileTreeItem({ node, depth, expandedPaths, onToggle, onFileClick }: FileTreeItemProps): JSX.Element {
  const isExpanded = expandedPaths.has(node.path)
  const isDirectory = node.type === 'directory'
  const paddingLeft = 12 + depth * 16

  const handleClick = () => {
    if (isDirectory) {
      onToggle(node.path)
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
            <FileTreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              expandedPaths={expandedPaths}
              onToggle={onToggle}
              onFileClick={onFileClick}
            />
          ))}
        </div>
      )}
    </div>
  )
}

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
  // 根据扩展名返回不同颜色
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
