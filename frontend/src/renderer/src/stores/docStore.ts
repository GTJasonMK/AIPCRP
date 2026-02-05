import { create } from 'zustand'
import {
  startDocGeneration,
  cancelTask,
  subscribeProgress,
  setBackendPort
} from '../services/docService'
import { FileTreeNode, useFileStore } from './fileStore'

interface TaskStats {
  total_files: number
  processed_files: number
  total_dirs: number
  processed_dirs: number
  failed_count: number
  skipped_count: number
}

// 文件生成状态
export type DocFileStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'skipped' | 'interrupted'

// 带状态的文档树节点
export interface DocTreeNode {
  name: string
  path: string
  sourcePath: string  // 对应的源码路径
  type: 'file' | 'directory'
  status: DocFileStatus
  children?: DocTreeNode[]
}

type TaskStatus = 'idle' | 'running' | 'completed' | 'failed' | 'cancelled'

interface DocStore {
  // State
  taskId: string | null
  status: TaskStatus
  progress: number
  currentFile: string | null
  stats: TaskStats | null
  docsPath: string | null
  error: string | null
  projectPath: string | null  // 项目根路径，用于路径转换

  // 文档树状态
  docsTree: DocTreeNode[]
  fileStatusMap: Map<string, DocFileStatus>  // sourcePath -> status
  relativeToSourceMap: Map<string, string>   // 相对路径 -> 完整源码路径

  // Actions
  startGeneration: (sourcePath: string, docsPath?: string, resume?: boolean) => Promise<void>
  cancelGeneration: () => Promise<void>
  reset: () => void
  initPort: (port: number) => void

  // 文档树相关
  generateDocsTreeFromSource: (sourceTree: FileTreeNode[], docsBasePath: string, projectPath: string) => void
  loadExistingDocsTree: (sourceTree: FileTreeNode[], docsBasePath: string, projectPath: string, existingDocPaths: Set<string>) => void
  updateFileStatus: (sourcePath: string, status: DocFileStatus) => void
  updateFileStatusByRelativePath: (relativePath: string, status: DocFileStatus) => void
}

const initialStats: TaskStats = {
  total_files: 0,
  processed_files: 0,
  total_dirs: 0,
  processed_dirs: 0,
  failed_count: 0,
  skipped_count: 0
}

let unsubscribe: (() => void) | null = null

// 支持的代码文件扩展名
const SUPPORTED_EXTENSIONS = new Set([
  'py', 'js', 'ts', 'jsx', 'tsx', 'java', 'go', 'rs',
  'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php', 'swift',
  'kt', 'scala', 'vue', 'svelte'
])

// 从源码树生成文档树结构，同时构建相对路径映射
function buildDocsTree(
  sourceNodes: FileTreeNode[],
  docsBasePath: string,
  projectPath: string,
  fileStatusMap: Map<string, DocFileStatus>,
  relativeToSourceMap: Map<string, string>
): DocTreeNode[] {
  const result: DocTreeNode[] = []

  // 规范化路径：统一为正斜杠，去除尾部斜杠
  const normalizedProjectPath = projectPath.replace(/\\/g, '/').replace(/\/$/, '')
  const normalizedDocsBasePath = docsBasePath.replace(/\\/g, '/').replace(/\/$/, '')

  for (const node of sourceNodes) {
    // 规范化节点路径
    const normalizedNodePath = node.path.replace(/\\/g, '/')

    // 计算相对路径：去除项目路径前缀
    let relativePath = ''
    if (normalizedNodePath.startsWith(normalizedProjectPath + '/')) {
      relativePath = normalizedNodePath.slice(normalizedProjectPath.length + 1)
    } else if (normalizedNodePath.startsWith(normalizedProjectPath)) {
      relativePath = normalizedNodePath.slice(normalizedProjectPath.length).replace(/^\//, '')
    } else {
      // 路径不匹配，尝试用最后一部分
      console.warn('[buildDocsTree] Path mismatch:', { normalizedNodePath, normalizedProjectPath })
      relativePath = node.name
    }

    if (node.type === 'directory') {
      // 目录节点
      const children = node.children
        ? buildDocsTree(node.children, docsBasePath, projectPath, fileStatusMap, relativeToSourceMap)
        : []

      // 只添加有子节点的目录
      if (children.length > 0) {
        const docPath = `${normalizedDocsBasePath}/${relativePath}/_summary.md`

        // 建立相对路径映射
        relativeToSourceMap.set(relativePath, node.path)

        result.push({
          name: node.name,
          path: docPath,
          sourcePath: node.path,
          type: 'directory',
          status: fileStatusMap.get(node.path) || 'pending',
          children
        })
      }
    } else {
      // 文件节点 - 只处理支持的扩展名
      const ext = node.name.split('.').pop()?.toLowerCase() || ''
      if (SUPPORTED_EXTENSIONS.has(ext)) {
        const docPath = `${normalizedDocsBasePath}/${relativePath}.md`

        // 建立相对路径映射
        relativeToSourceMap.set(relativePath, node.path)

        result.push({
          name: `${node.name}.md`,
          path: docPath,
          sourcePath: node.path,
          type: 'file',
          status: fileStatusMap.get(node.path) || 'pending'
        })
      }
    }
  }

  return result
}

// 更新文档树中节点的状态
function updateTreeNodeStatus(
  nodes: DocTreeNode[],
  sourcePath: string,
  status: DocFileStatus
): DocTreeNode[] {
  return nodes.map(node => {
    // 先递归更新子节点（如果有的话）
    const updatedChildren = node.children
      ? updateTreeNodeStatus(node.children, sourcePath, status)
      : undefined

    // 如果当前节点匹配，更新状态
    if (node.sourcePath === sourcePath) {
      return { ...node, status, children: updatedChildren }
    }

    // 如果子节点有更新，返回新对象
    if (updatedChildren) {
      return { ...node, children: updatedChildren }
    }

    return node
  })
}

// 将所有 processing 状态的节点标记为 interrupted
function markProcessingAsInterrupted(nodes: DocTreeNode[]): DocTreeNode[] {
  return nodes.map(node => {
    const updatedChildren = node.children
      ? markProcessingAsInterrupted(node.children)
      : undefined

    if (node.status === 'processing') {
      return { ...node, status: 'interrupted' as DocFileStatus, children: updatedChildren }
    }

    if (updatedChildren) {
      return { ...node, children: updatedChildren }
    }

    return node
  })
}

export const useDocStore = create<DocStore>((set, get) => ({
  taskId: null,
  status: 'idle',
  progress: 0,
  currentFile: null,
  stats: null,
  docsPath: null,
  error: null,
  projectPath: null,
  docsTree: [],
  fileStatusMap: new Map(),
  relativeToSourceMap: new Map(),

  initPort: (port: number) => {
    setBackendPort(port)
  },

  generateDocsTreeFromSource: (sourceTree: FileTreeNode[], docsBasePath: string, projectPath: string) => {
    const fileStatusMap = new Map<string, DocFileStatus>()
    const relativeToSourceMap = new Map<string, string>()
    const docsTree = buildDocsTree(sourceTree, docsBasePath, projectPath, fileStatusMap, relativeToSourceMap)
    console.log('[docStore] generateDocsTreeFromSource:', {
      docsTreeLength: docsTree.length,
      relativeToSourceMapSize: relativeToSourceMap.size,
      projectPath,
      docsBasePath,
      sourceTreeLength: sourceTree.length,
      // 打印前10个映射键值对
      mapEntries: Array.from(relativeToSourceMap.entries()).slice(0, 10)
    })
    set({ docsTree, fileStatusMap, relativeToSourceMap, docsPath: docsBasePath, projectPath })
  },

  // 加载已存在的文档目录（根据实际存在的文件设置状态）
  loadExistingDocsTree: (sourceTree: FileTreeNode[], docsBasePath: string, projectPath: string, existingDocPaths: Set<string>) => {
    const fileStatusMap = new Map<string, DocFileStatus>()
    const relativeToSourceMap = new Map<string, string>()
    const docsTree = buildDocsTree(sourceTree, docsBasePath, projectPath, fileStatusMap, relativeToSourceMap)

    // 根据实际存在的文档文件设置状态
    // 目录的状态根据子节点决定：全部完成则完成，有未完成则 pending
    const markByExistence = (nodes: DocTreeNode[]): DocTreeNode[] => {
      return nodes.map(node => {
        if (node.type === 'directory') {
          // 先递归处理子节点
          const updatedChildren = node.children ? markByExistence(node.children) : undefined

          // 目录状态根据子节点决定
          let status: DocFileStatus = 'completed'
          if (updatedChildren && updatedChildren.length > 0) {
            const hasIncomplete = updatedChildren.some(child => child.status !== 'completed')
            status = hasIncomplete ? 'pending' : 'completed'
          }

          return {
            ...node,
            status,
            children: updatedChildren
          }
        } else {
          // 文件：检查文档是否存在
          const normalizedPath = node.path.replace(/\\/g, '/')
          const exists = existingDocPaths.has(normalizedPath)
          const status: DocFileStatus = exists ? 'completed' : 'pending'

          return {
            ...node,
            status
          }
        }
      })
    }
    const markedTree = markByExistence(docsTree)

    // 统计已完成和待处理的数量（只统计文件）
    let completedCount = 0
    let pendingCount = 0
    const countStatus = (nodes: DocTreeNode[]): void => {
      for (const node of nodes) {
        if (node.type === 'file') {
          if (node.status === 'completed') completedCount++
          else pendingCount++
        }
        if (node.children) countStatus(node.children)
      }
    }
    countStatus(markedTree)

    console.log('[docStore] loadExistingDocsTree:', {
      docsTreeLength: markedTree.length,
      projectPath,
      docsBasePath,
      existingDocPathsSize: existingDocPaths.size,
      completedCount,
      pendingCount
    })

    // 根据是否有未完成的文件来设置状态
    const hasIncomplete = pendingCount > 0
    set({
      docsTree: markedTree,
      fileStatusMap,
      relativeToSourceMap,
      docsPath: docsBasePath,
      projectPath,
      status: hasIncomplete ? 'idle' : 'completed',
      progress: hasIncomplete ? Math.round((completedCount / (completedCount + pendingCount)) * 100) : 100
    })
  },

  updateFileStatus: (sourcePath: string, status: DocFileStatus) => {
    const { docsTree, fileStatusMap } = get()
    console.log('[docStore] updateFileStatus:', { sourcePath, status, docsTreeLength: docsTree.length })
    const newMap = new Map(fileStatusMap)
    newMap.set(sourcePath, status)
    const newTree = updateTreeNodeStatus(docsTree, sourcePath, status)
    console.log('[docStore] updateFileStatus - newTree length:', newTree.length)
    set({ docsTree: newTree, fileStatusMap: newMap })
  },

  // 通过相对路径更新状态（后端发送的路径是相对路径）
  updateFileStatusByRelativePath: (relativePath: string, status: DocFileStatus) => {
    const { relativeToSourceMap, docsTree } = get()
    // 调试日志：打印收到的路径和映射内容
    console.log('[docStore] updateFileStatusByRelativePath called:', {
      relativePath,
      status,
      mapSize: relativeToSourceMap.size,
      docsTreeLength: docsTree.length,
      // 打印映射中的前5个键
      mapKeys: Array.from(relativeToSourceMap.keys()).slice(0, 5)
    })
    // 将相对路径转换为完整源码路径
    const sourcePath = relativeToSourceMap.get(relativePath)
    if (sourcePath) {
      console.log('[docStore] Found sourcePath:', sourcePath)
      get().updateFileStatus(sourcePath, status)
    } else {
      console.warn('[docStore] 路径映射未找到:', relativePath, '| 映射大小:', relativeToSourceMap.size)
      // 尝试打印所有映射键来调试
      if (relativeToSourceMap.size < 20) {
        console.log('[docStore] 所有映射键:', Array.from(relativeToSourceMap.keys()))
      }
    }
  },

  startGeneration: async (sourcePath: string, docsPath?: string, resume?: boolean) => {
    // Clean up previous subscription
    if (unsubscribe) {
      unsubscribe()
      unsubscribe = null
    }

    // 在重置状态之前，先获取 fileStore 的状态
    // 因为 sourcePath 就是 projectPath，用它来验证
    const fileStoreState = useFileStore.getState()
    console.log('[docStore] startGeneration - fileStore state:', {
      fileTreeLength: fileStoreState.fileTree.length,
      projectPath: fileStoreState.projectPath,
      sourcePath
    })

    set({
      status: 'running',
      progress: 0,
      currentFile: null,
      stats: initialStats,
      error: null,
      docsTree: [],
      fileStatusMap: new Map(),
      relativeToSourceMap: new Map(),
      projectPath: sourcePath  // 直接使用 sourcePath 作为 projectPath
    })

    try {
      const response = await startDocGeneration(sourcePath, docsPath, resume)
      console.log('[docStore] startGeneration - API response:', response)

      set({
        taskId: response.task_id,
        docsPath: response.docs_path
      })

      // 在建立 WebSocket 订阅之前，立即生成文档树和路径映射
      // 这样 WebSocket 消息到来时映射已就绪
      // 使用 sourcePath 作为 projectPath（它们应该相同）
      if (fileStoreState.fileTree.length > 0) {
        console.log('[docStore] startGeneration - 生成文档树，使用 sourcePath:', sourcePath)
        get().generateDocsTreeFromSource(
          fileStoreState.fileTree,
          response.docs_path,
          sourcePath  // 直接使用 sourcePath，更可靠
        )
      } else {
        console.warn('[docStore] startGeneration - fileTree 为空，跳过文档树生成')
      }

      // Subscribe to progress updates
      unsubscribe = subscribeProgress(response.task_id, {
        onProgress: (progress, currentFile, stats) => {
          set({ progress, currentFile, stats })
        },
        onFileStarted: (path) => {
          console.log('File started:', path)
          get().updateFileStatusByRelativePath(path, 'processing')
        },
        onFileCompleted: (path) => {
          console.log('File completed:', path)
          get().updateFileStatusByRelativePath(path, 'completed')
        },
        onDirStarted: (path) => {
          console.log('Directory started:', path)
          get().updateFileStatusByRelativePath(path, 'processing')
        },
        onDirCompleted: (path) => {
          console.log('Directory completed:', path)
          get().updateFileStatusByRelativePath(path, 'completed')
        },
        onCompleted: (stats) => {
          set({
            status: 'completed',
            progress: 100,
            currentFile: null,
            stats
          })
          if (unsubscribe) {
            unsubscribe()
            unsubscribe = null
          }
        },
        onError: (message) => {
          set({
            status: 'failed',
            error: message
          })
          if (unsubscribe) {
            unsubscribe()
            unsubscribe = null
          }
        },
        onCancelled: () => {
          // 将所有正在处理的文件标记为 interrupted（黄色）
          // 这些文件下次生成时会重新处理
          const { docsTree } = get()
          const updatedTree = markProcessingAsInterrupted(docsTree)
          set({
            status: 'cancelled',
            docsTree: updatedTree
          })
          if (unsubscribe) {
            unsubscribe()
            unsubscribe = null
          }
        }
      })
    } catch (error) {
      set({
        status: 'failed',
        error: error instanceof Error ? error.message : 'Unknown error'
      })
    }
  },

  cancelGeneration: async () => {
    const { taskId, docsTree } = get()
    if (!taskId) return

    try {
      await cancelTask(taskId)
      // 将所有正在处理的文件标记为 interrupted（橙色）
      const updatedTree = markProcessingAsInterrupted(docsTree)
      set({ status: 'cancelled', docsTree: updatedTree })
    } catch (error) {
      console.error('Failed to cancel task:', error)
    }

    if (unsubscribe) {
      unsubscribe()
      unsubscribe = null
    }
  },

  reset: () => {
    if (unsubscribe) {
      unsubscribe()
      unsubscribe = null
    }
    set({
      taskId: null,
      status: 'idle',
      progress: 0,
      currentFile: null,
      stats: null,
      docsPath: null,
      error: null,
      projectPath: null,
      docsTree: [],
      fileStatusMap: new Map(),
      relativeToSourceMap: new Map()
    })
  }
}))
