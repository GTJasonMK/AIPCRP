import { create } from 'zustand'

export interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

// 最近项目
export interface RecentProject {
  path: string
  name: string
  lastOpened: number
}

// 视图模式：仅源码 / 源码+文档并排
export type ViewMode = 'code-only' | 'code-and-docs'

interface FileStore {
  // 状态
  projectPath: string | null
  fileTree: FileTreeNode[]
  expandedPaths: Set<string>
  isLoading: boolean

  // 最近项目
  recentProjects: RecentProject[]

  // 文档相关状态
  docsPath: string | null
  docsTree: FileTreeNode[]
  docsExpandedPaths: Set<string>

  // 视图模式
  viewMode: ViewMode

  // 当前选中的目录（用于显示目录总结）
  selectedDirPath: string | null

  // 动作
  setProjectPath: (path: string | null) => void
  setFileTree: (tree: FileTreeNode[]) => void
  toggleExpanded: (path: string) => void
  loadProject: () => Promise<void>
  openProjectByPath: (projectPath: string) => Promise<void>
  loadRecentProjects: () => Promise<void>
  removeRecentProject: (projectPath: string) => Promise<void>

  // 文档相关动作
  setDocsPath: (path: string | null) => void
  setDocsTree: (tree: FileTreeNode[]) => void
  toggleDocsExpanded: (path: string) => void
  loadDocsTree: (docsPath: string) => Promise<void>
  expandAllDocsPaths: (paths: string[]) => void

  // 视图模式动作
  setViewMode: (mode: ViewMode) => void

  // 目录选择
  setSelectedDirPath: (path: string | null) => void
}

// 文档目录名（固定为 .docs）
const DOCS_FOLDER_NAME = '.docs'

// 过滤文档目录（从源码树中排除 .docs 目录）
function filterDocsFolder(tree: FileTreeNode[]): FileTreeNode[] {
  return tree.filter(node => {
    // 过滤掉 .docs 目录
    if (node.type === 'directory' && node.name === DOCS_FOLDER_NAME) {
      return false
    }
    if (node.children) {
      node.children = filterDocsFolder(node.children)
    }
    return true
  })
}

export const useFileStore = create<FileStore>((set, get) => ({
  projectPath: null,
  fileTree: [],
  expandedPaths: new Set(),
  isLoading: false,

  recentProjects: [],

  docsPath: null,
  docsTree: [],
  docsExpandedPaths: new Set(),

  viewMode: 'code-only',

  selectedDirPath: null,

  setProjectPath: (path) => set({ projectPath: path }),

  setFileTree: (tree) => set({ fileTree: tree }),

  toggleExpanded: (path) => {
    const { expandedPaths } = get()
    const newExpanded = new Set(expandedPaths)
    if (newExpanded.has(path)) {
      newExpanded.delete(path)
    } else {
      newExpanded.add(path)
    }
    set({ expandedPaths: newExpanded })
  },

  loadProject: async () => {
    set({ isLoading: true })
    try {
      const selectedPath = await window.api.selectDirectory()
      if (selectedPath) {
        // 复用 openProjectByPath 逻辑
        await get().openProjectByPath(selectedPath)
      }
    } catch (error) {
      console.error('加载项目失败:', error)
    } finally {
      set({ isLoading: false })
    }
  },

  openProjectByPath: async (projectPath: string) => {
    set({ isLoading: true })
    try {
      set({ projectPath })
      const tree = await window.api.getFileTree(projectPath)

      console.log('[fileStore] openProjectByPath - top level items:', tree.map(n => ({ name: n.name, type: n.type })))

      // 检查是否存在 .docs 目录（在过滤前检查）
      const docsFolder = tree.find(node => node.type === 'directory' && node.name === DOCS_FOLDER_NAME)
      console.log('[fileStore] openProjectByPath - docsFolder found:', docsFolder?.path || 'not found')

      if (docsFolder) {
        set({ docsPath: docsFolder.path })
      } else {
        set({ docsPath: null })
      }

      // 过滤掉 .docs 目录，使其不在 Source 栏显示
      const filteredTree = filterDocsFolder(tree)
      set({ fileTree: filteredTree, expandedPaths: new Set() })

      // 记录到最近项目列表
      const updatedProjects = await window.api.addRecentProject(projectPath)
      set({ recentProjects: updatedProjects })
    } catch (error) {
      console.error('打开项目失败:', error)
    } finally {
      set({ isLoading: false })
    }
  },

  loadRecentProjects: async () => {
    try {
      const projects = await window.api.getRecentProjects()
      set({ recentProjects: projects })
    } catch (error) {
      console.error('加载最近项目失败:', error)
    }
  },

  removeRecentProject: async (projectPath: string) => {
    try {
      const updatedProjects = await window.api.removeRecentProject(projectPath)
      set({ recentProjects: updatedProjects })
    } catch (error) {
      console.error('移除最近项目失败:', error)
    }
  },

  setDocsPath: (path) => {
    set({ docsPath: path })
  },

  setDocsTree: (tree) => set({ docsTree: tree }),

  toggleDocsExpanded: (path) => {
    const { docsExpandedPaths } = get()
    const newExpanded = new Set(docsExpandedPaths)
    if (newExpanded.has(path)) {
      newExpanded.delete(path)
    } else {
      newExpanded.add(path)
    }
    set({ docsExpandedPaths: newExpanded })
  },

  loadDocsTree: async (docsPath: string) => {
    try {
      const tree = await window.api.getFileTree(docsPath)
      set({ docsTree: tree, docsExpandedPaths: new Set(), docsPath })
    } catch (error) {
      console.error('加载文档目录失败:', error)
    }
  },

  expandAllDocsPaths: (paths: string[]) => {
    set({ docsExpandedPaths: new Set(paths) })
  },

  setViewMode: (mode) => set({ viewMode: mode }),

  setSelectedDirPath: (path) => set({ selectedDirPath: path })
}))
