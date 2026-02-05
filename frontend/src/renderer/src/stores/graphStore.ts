import { create } from 'zustand'

interface GraphNodeData {
  id: string
  label: string
  type: string // file, class, function, method, interface, directory
  file_path?: string
  line_number?: number
  line?: number // LLM 图谱使用 line 而不是 line_number
  metadata?: Record<string, string>
}

interface GraphEdgeData {
  source: string
  target: string
  type: string // imports, calls, inherits, contains
  label?: string
}

type GraphScope = 'project' | 'module'

// 图谱来源：静态分析 或 LLM 提取
type GraphSource = 'static' | 'llm'

// LLM 项目图谱响应格式
interface LLMProjectGraphResponse {
  project_name: string
  file_count: number
  nodes: GraphNodeData[]
  edges: GraphEdgeData[]
  generated_at: string
}

// LLM 文件图谱响应格式
interface LLMFileGraphResponse {
  file_path: string
  file_id: string
  nodes: GraphNodeData[]
  edges: GraphEdgeData[]
  imports: Array<{ module: string; names: string[] }>
}

// LLM 目录图谱响应格式
interface LLMDirGraphResponse {
  dir_path: string
  dir_id: string
  nodes: GraphNodeData[]
  edges: GraphEdgeData[]
  imports: Array<{ module: string; items: string[] }>
}

interface GraphStore {
  // State
  nodes: GraphNodeData[]
  edges: GraphEdgeData[]
  scope: GraphScope
  loading: boolean
  selectedFilePath: string | null
  graphSource: GraphSource
  llmGraphAvailable: boolean // LLM 图谱是否可用

  // Actions
  loadProjectGraph: (projectPath: string) => Promise<void>
  loadModuleGraph: (projectPath: string, filePath: string) => Promise<void>
  loadLLMProjectGraph: (docsPath: string) => Promise<boolean>
  loadLLMFileGraph: (docsPath: string, filePath: string) => Promise<boolean>
  loadLLMDirGraph: (docsPath: string, dirPath: string) => Promise<boolean>
  setScope: (scope: GraphScope) => void
  setGraphSource: (source: GraphSource) => void
  clear: () => void
}

export const useGraphStore = create<GraphStore>((set, get) => ({
  nodes: [],
  edges: [],
  scope: 'project',
  loading: false,
  selectedFilePath: null,
  graphSource: 'static',
  llmGraphAvailable: false,

  loadProjectGraph: async (projectPath: string) => {
    set({ loading: true })
    try {
      const port = await window.api.getBackendPort()
      const res = await fetch(`http://127.0.0.1:${port}/api/graph/project`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_path: projectPath })
      })
      if (res.ok) {
        const data = await res.json()
        set({
          nodes: data.nodes,
          edges: data.edges,
          scope: 'project',
          selectedFilePath: null,
          graphSource: 'static'
        })
      } else {
        console.error('Failed to load project graph:', await res.text())
      }
    } catch (error) {
      console.error('Failed to load project graph:', error)
    } finally {
      set({ loading: false })
    }
  },

  loadModuleGraph: async (projectPath: string, filePath: string) => {
    set({ loading: true })
    try {
      const port = await window.api.getBackendPort()
      const res = await fetch(`http://127.0.0.1:${port}/api/graph/module`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_path: projectPath, file_path: filePath })
      })
      if (res.ok) {
        const data = await res.json()
        set({
          nodes: data.nodes,
          edges: data.edges,
          scope: 'module',
          selectedFilePath: filePath
        })
      } else {
        console.error('Failed to load module graph:', await res.text())
      }
    } catch (error) {
      console.error('Failed to load module graph:', error)
    } finally {
      set({ loading: false })
    }
  },

  // 加载 LLM 生成的项目图谱
  loadLLMProjectGraph: async (docsPath: string): Promise<boolean> => {
    set({ loading: true })
    try {
      const port = await window.api.getBackendPort()
      const res = await fetch(`http://127.0.0.1:${port}/api/docs/graph`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ docs_path: docsPath })
      })
      if (res.ok) {
        const data: LLMProjectGraphResponse = await res.json()
        // 规范化节点数据：将 line 转换为 line_number
        const normalizedNodes = data.nodes.map(node => ({
          ...node,
          line_number: node.line_number ?? node.line
        }))
        set({
          nodes: normalizedNodes,
          edges: data.edges,
          scope: 'project',
          selectedFilePath: null,
          graphSource: 'llm',
          llmGraphAvailable: true
        })
        console.log(`[graphStore] LLM 图谱已加载: ${data.nodes.length} 节点, ${data.edges.length} 边`)
        return true
      } else {
        const errorText = await res.text()
        console.warn('[graphStore] LLM 图谱不可用:', errorText)
        set({ llmGraphAvailable: false })
        return false
      }
    } catch (error) {
      console.error('[graphStore] 加载 LLM 图谱失败:', error)
      set({ llmGraphAvailable: false })
      return false
    } finally {
      set({ loading: false })
    }
  },

  // 加载 LLM 生成的单文件图谱
  loadLLMFileGraph: async (docsPath: string, filePath: string): Promise<boolean> => {
    set({ loading: true })
    try {
      const port = await window.api.getBackendPort()
      const res = await fetch(`http://127.0.0.1:${port}/api/docs/file-graph`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ docs_path: docsPath, file_path: filePath })
      })
      if (res.ok) {
        const data: LLMFileGraphResponse = await res.json()
        // 规范化节点数据：将 line 转换为 line_number
        const normalizedNodes = data.nodes.map(node => ({
          ...node,
          line_number: node.line_number ?? node.line
        }))
        set({
          nodes: normalizedNodes,
          edges: data.edges,
          scope: 'module',
          selectedFilePath: filePath,
          graphSource: 'llm',
          llmGraphAvailable: true
        })
        console.log(`[graphStore] LLM 文件图谱已加载: ${filePath}, ${data.nodes.length} 节点, ${data.edges.length} 边`)
        return true
      } else {
        const errorText = await res.text()
        console.warn('[graphStore] LLM 文件图谱不可用:', errorText)
        return false
      }
    } catch (error) {
      console.error('[graphStore] 加载 LLM 文件图谱失败:', error)
      return false
    } finally {
      set({ loading: false })
    }
  },

  // 加载 LLM 生成的目录图谱
  loadLLMDirGraph: async (docsPath: string, dirPath: string): Promise<boolean> => {
    set({ loading: true })
    try {
      const port = await window.api.getBackendPort()
      const res = await fetch(`http://127.0.0.1:${port}/api/docs/dir-graph`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ docs_path: docsPath, dir_path: dirPath })
      })
      if (res.ok) {
        const data: LLMDirGraphResponse = await res.json()
        // 规范化节点数据：将 line 转换为 line_number
        const normalizedNodes = data.nodes.map(node => ({
          ...node,
          line_number: node.line_number ?? node.line
        }))
        set({
          nodes: normalizedNodes,
          edges: data.edges,
          scope: 'module',
          selectedFilePath: dirPath || '(root)',
          graphSource: 'llm',
          llmGraphAvailable: true
        })
        console.log(`[graphStore] LLM 目录图谱已加载: ${dirPath || '(root)'}, ${data.nodes.length} 节点, ${data.edges.length} 边`)
        return true
      } else {
        const errorText = await res.text()
        console.warn('[graphStore] LLM 目录图谱不可用:', errorText)
        return false
      }
    } catch (error) {
      console.error('[graphStore] 加载 LLM 目录图谱失败:', error)
      return false
    } finally {
      set({ loading: false })
    }
  },

  setScope: (scope: GraphScope) => set({ scope }),

  setGraphSource: (source: GraphSource) => set({ graphSource: source }),

  clear: () => set({
    nodes: [],
    edges: [],
    selectedFilePath: null,
    llmGraphAvailable: false
  })
}))
