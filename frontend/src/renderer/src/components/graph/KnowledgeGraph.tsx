import { useEffect, useRef } from 'react'
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  Node,
  Edge,
  useNodesState,
  useEdgesState,
  MarkerType,
  Position
} from 'reactflow'
import Dagre from '@dagrejs/dagre'
import 'reactflow/dist/style.css'
import { useGraphStore } from '../../stores/graphStore'
import { useDocStore } from '../../stores/docStore'
import { useFileStore } from '../../stores/fileStore'
import { useEditorStore } from '../../stores/editorStore'
import { CustomNode } from './CustomNode'

// Node type color mapping
const NODE_COLORS: Record<string, string> = {
  file: '#3b82f6',       // blue
  directory: '#6b7280',  // gray
  class: '#8b5cf6',      // purple
  interface: '#06b6d4',  // cyan
  function: '#22c55e',   // green
  method: '#84cc16',     // lime
  module: '#f59e0b',     // amber
  struct: '#f97316',     // orange
  enum: '#ec4899',       // pink
  constant: '#14b8a6'    // teal
}

// Edge type style mapping
const EDGE_STYLES: Record<string, { stroke: string; strokeDasharray?: string }> = {
  imports: { stroke: '#3b82f6' },
  calls: { stroke: '#22c55e', strokeDasharray: '5 5' },
  inherits: { stroke: '#8b5cf6' },
  implements: { stroke: '#06b6d4' },
  contains: { stroke: '#6b7280', strokeDasharray: '3 3' },
  depends: { stroke: '#f59e0b', strokeDasharray: '5 5' }
}

const nodeTypes = { custom: CustomNode }

export function KnowledgeGraph(): JSX.Element {
  const {
    nodes: graphNodes,
    edges: graphEdges,
    scope,
    loading,
    selectedFilePath,
    loadLLMFileGraph,
    loadLLMDirGraph
  } = useGraphStore()
  const { docsPath: docStoreDocsPath } = useDocStore()
  const { docsPath: fileStoreDocsPath, projectPath } = useFileStore()
  const { activeFile, selectedDirSummary } = useEditorStore()

  // 优先使用 docStore 的 docsPath，否则使用 fileStore 的
  const docsPath = docStoreDocsPath || fileStoreDocsPath

  const [rfNodes, setRfNodes, onNodesChange] = useNodesState([])
  const [rfEdges, setRfEdges, onEdgesChange] = useEdgesState([])

  // 用于跟踪上一次加载的路径，避免重复加载
  const lastLoadedPathRef = useRef<string | null>(null)

  // 当 activeFile 变化时加载对应的 LLM 文件图谱
  useEffect(() => {
    // 如果有选中的目录总结，优先显示目录图谱（由下一个 useEffect 处理）
    if (selectedDirSummary) return
    if (!docsPath || !activeFile || !projectPath || loading) return

    // 计算相对路径
    const relativePath = activeFile.path
      .replace(/\\/g, '/')
      .replace(projectPath.replace(/\\/g, '/'), '')
      .replace(/^\//, '')

    // 如果已经加载过这个路径，跳过
    if (lastLoadedPathRef.current === `file:${relativePath}`) return

    lastLoadedPathRef.current = `file:${relativePath}`
    loadLLMFileGraph(docsPath, relativePath)
  }, [docsPath, activeFile, projectPath, loading, loadLLMFileGraph, selectedDirSummary])

  // 当 selectedDirSummary 变化时加载对应的 LLM 目录图谱
  useEffect(() => {
    if (!docsPath || !selectedDirSummary || !projectPath || loading) return

    // 计算相对路径
    const relativePath = selectedDirSummary.path
      .replace(/\\/g, '/')
      .replace(projectPath.replace(/\\/g, '/'), '')
      .replace(/^\//, '')

    // 如果已经加载过这个路径，跳过
    if (lastLoadedPathRef.current === `dir:${relativePath}`) return

    lastLoadedPathRef.current = `dir:${relativePath}`
    loadLLMDirGraph(docsPath, relativePath)
  }, [docsPath, selectedDirSummary, projectPath, loading, loadLLMDirGraph])

  // Convert backend data to React Flow format
  useEffect(() => {
    if (graphNodes.length === 0) {
      setRfNodes([])
      setRfEdges([])
      return
    }

    // Filter out directory nodes for cleaner layout
    const visibleNodes = graphNodes.filter(n => n.type !== 'directory')

    // 使用层次化布局算法，传入边信息以优化节点排列
    const nodes: Node[] = layoutNodes(visibleNodes, scope, graphEdges)
    const edges: Edge[] = graphEdges
      .filter(e => {
        // Only show edges whose source and target are visible
        const nodeIds = new Set(visibleNodes.map(n => n.id))
        return nodeIds.has(e.source) && nodeIds.has(e.target)
      })
      .map(e => {
        const style = EDGE_STYLES[e.type] || EDGE_STYLES.contains
        return {
          id: `${e.source}-${e.type}-${e.target}`,
          source: e.source,
          target: e.target,
          label: e.label || e.type,
          type: 'smoothstep',
          animated: e.type === 'calls',
          style: { ...style, strokeWidth: 1.5 },
          labelStyle: { fontSize: 10, fill: '#9ca3af' },
          // 增加边的路径偏移，避免穿过节点
          pathOptions: { offset: 30, borderRadius: 15 },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: style.stroke,
            width: 15,
            height: 15
          }
        }
      })

    setRfNodes(nodes)
    setRfEdges(edges)
  }, [graphNodes, graphEdges, scope])

  const hasGraph = rfNodes.length > 0
  const hasSelection = activeFile || selectedDirSummary

  return (
    <div className="h-full flex flex-col bg-[#1e1e1e]">
      {/* Status Bar */}
      <div className="h-7 flex items-center px-3 bg-[#252526] border-b border-[#3c3c3c] shrink-0">
        <span className="text-xs text-gray-500">
          {loading ? (
            'Loading graph...'
          ) : hasGraph ? (
            <>
              {selectedFilePath && <span className="text-gray-400 mr-2">{selectedFilePath}</span>}
              {rfNodes.length} nodes, {rfEdges.length} edges
            </>
          ) : hasSelection ? (
            selectedDirSummary
              ? 'No graph data for this directory'
              : 'No graph data for this file'
          ) : docsPath ? (
            'Select a file or directory to view its knowledge graph'
          ) : (
            'Generate docs to view knowledge graph'
          )}
        </span>
      </div>

      {/* Graph Area */}
      <div className="flex-1">
        {!hasGraph && !loading ? (
          <div className="h-full flex items-center justify-center text-gray-500 text-sm">
            {!docsPath
              ? 'Generate documentation first to view knowledge graph'
              : !hasSelection
                ? 'Select a file or directory to view its knowledge graph'
                : selectedDirSummary
                  ? 'No knowledge graph data for this directory'
                  : 'No knowledge graph data for this file'
            }
          </div>
        ) : (
          <ReactFlow
            nodes={rfNodes}
            edges={rfEdges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            nodeTypes={nodeTypes}
            fitView
            fitViewOptions={{ padding: 0.2 }}
            minZoom={0.1}
            maxZoom={3}
            proOptions={{ hideAttribution: true }}
          >
            <Background color="#333" gap={20} />
            <Controls
              showInteractive={false}
              style={{ backgroundColor: '#252526', borderColor: '#3c3c3c' }}
            />
            <MiniMap
              nodeColor={(n) => NODE_COLORS[n.data?.type] || '#6b7280'}
              maskColor="rgba(0,0,0,0.7)"
              style={{ backgroundColor: '#1e1e1e', borderColor: '#3c3c3c' }}
            />
          </ReactFlow>
        )}
      </div>
    </div>
  )
}

// ========================
// 使用 Dagre 进行图布局
// Dagre 是专业的有向图布局库，能有效最小化边交叉
// ========================

interface BackendNode {
  id: string
  label: string
  type: string
  file_path?: string
  line_number?: number
  metadata?: Record<string, string>
}

interface BackendEdge {
  source: string
  target: string
  type: string
}

// 布局配置
const LAYOUT_CONFIG = {
  nodeWidth: 160,
  nodeHeight: 44,
  rankSep: 120,     // 层间距离
  nodeSep: 80,      // 同层节点间距
  edgeSep: 40,      // 边间距
  rankDir: 'TB'     // 布局方向: TB(上到下), LR(左到右)
}

/**
 * 使用 Dagre 进行图布局
 * 参考: https://reactflow.dev/examples/layout/dagre
 */
function layoutNodes(
  nodes: BackendNode[],
  _scope: string,
  edges: BackendEdge[] = []
): Node[] {
  if (nodes.length === 0) return []

  const { nodeWidth, nodeHeight, rankSep, nodeSep, edgeSep, rankDir } = LAYOUT_CONFIG

  // 创建 dagre 图
  const g = new Dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}))

  // 设置图的全局属性
  g.setGraph({
    rankdir: rankDir,
    ranksep: rankSep,
    nodesep: nodeSep,
    edgesep: edgeSep,
    marginx: 50,
    marginy: 50
  })

  // 添加节点到 dagre 图
  for (const node of nodes) {
    g.setNode(node.id, {
      width: nodeWidth,
      height: nodeHeight,
      label: node.label
    })
  }

  // 添加边到 dagre 图
  for (const edge of edges) {
    // 检查源和目标节点都存在
    if (g.hasNode(edge.source) && g.hasNode(edge.target)) {
      g.setEdge(edge.source, edge.target)
    }
  }

  // 执行布局计算
  Dagre.layout(g)

  // 转换为 ReactFlow 节点格式
  const isHorizontal = rankDir === 'LR'
  const result: Node[] = []

  for (const node of nodes) {
    const dagreNode = g.node(node.id)
    if (!dagreNode) continue

    result.push({
      id: node.id,
      type: 'custom',
      // dagre 返回的是中心点坐标，需要转换为左上角坐标
      position: {
        x: dagreNode.x - nodeWidth / 2,
        y: dagreNode.y - nodeHeight / 2
      },
      // 设置连接点位置
      targetPosition: isHorizontal ? Position.Left : Position.Top,
      sourcePosition: isHorizontal ? Position.Right : Position.Bottom,
      data: {
        label: node.label,
        type: node.type,
        file_path: node.file_path,
        line_number: node.line_number,
        color: NODE_COLORS[node.type] || '#6b7280',
        metadata: node.metadata
      }
    })
  }

  return result
}
