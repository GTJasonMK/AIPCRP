import { useCallback, useEffect, useRef, useState } from 'react'
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

// Edge type style mapping - 优化后更易区分
// 结构关系（contains）使用细虚线，逻辑关系（calls/imports）使用粗实线
const EDGE_STYLES: Record<string, { stroke: string; strokeDasharray?: string; strokeWidth?: number; opacity?: number }> = {
  imports: { stroke: '#3b82f6', strokeWidth: 2 },                          // 蓝色粗实线
  calls: { stroke: '#22c55e', strokeDasharray: '8 4', strokeWidth: 2 },    // 绿色粗虚线
  inherits: { stroke: '#a855f7', strokeWidth: 2.5 },                       // 紫色最粗实线
  implements: { stroke: '#06b6d4', strokeWidth: 2 },                       // 青色粗实线
  contains: { stroke: '#6b7280', strokeDasharray: '2 2', strokeWidth: 1, opacity: 0.5 },  // 灰色细点线（淡化）
  depends: { stroke: '#f59e0b', strokeDasharray: '6 3', strokeWidth: 1.5 } // 橙色中虚线
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

  // 关联高亮功能：保存原始节点和边的引用
  const baseNodesRef = useRef<Node[]>([])
  const baseEdgesRef = useRef<Edge[]>([])
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null)

  // 悬停事件处理
  const onNodeMouseEnter = useCallback((_: React.MouseEvent, node: Node) => {
    setHoveredNodeId(node.id)
  }, [])

  const onNodeMouseLeave = useCallback(() => {
    setHoveredNodeId(null)
  }, [])

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
      baseNodesRef.current = []
      baseEdgesRef.current = []
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
        const isStructural = e.type === 'contains'  // 结构关系淡化显示
        return {
          id: `${e.source}-${e.type}-${e.target}`,
          source: e.source,
          target: e.target,
          label: isStructural ? '' : (e.label || e.type),  // 结构关系不显示标签
          type: 'smoothstep',
          animated: e.type === 'calls',
          style: {
            stroke: style.stroke,
            strokeWidth: style.strokeWidth || 1.5,
            strokeDasharray: style.strokeDasharray,
            opacity: style.opacity || 1
          },
          labelStyle: { fontSize: 10, fill: '#9ca3af', fontWeight: 500 },
          labelBgStyle: { fill: '#1e1e1e', fillOpacity: 0.8 },
          labelBgPadding: [4, 2] as [number, number],
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: style.stroke,
            width: isStructural ? 10 : 15,  // 结构关系箭头更小
            height: isStructural ? 10 : 15
          }
        }
      })

    // 保存原始数据用于高亮效果
    baseNodesRef.current = nodes
    baseEdgesRef.current = edges
    setRfNodes(nodes)
    setRfEdges(edges)
  }, [graphNodes, graphEdges, scope])

  // 关联高亮效果：悬停节点时淡化不相关的节点和边
  useEffect(() => {
    const baseNodes = baseNodesRef.current
    const baseEdges = baseEdgesRef.current

    if (!hoveredNodeId || baseNodes.length === 0) {
      // 恢复原始样式
      if (baseNodes.length > 0) {
        setRfNodes(baseNodes)
        setRfEdges(baseEdges)
      }
      return
    }

    // 计算与悬停节点直接关联的节点和边
    const connectedNodeIds = new Set<string>([hoveredNodeId])
    const connectedEdgeIds = new Set<string>()

    for (const edge of baseEdges) {
      if (edge.source === hoveredNodeId || edge.target === hoveredNodeId) {
        connectedEdgeIds.add(edge.id)
        connectedNodeIds.add(edge.source)
        connectedNodeIds.add(edge.target)
      }
    }

    // 更新节点样式：关联节点保持原样，其他淡化
    setRfNodes(baseNodes.map(node => ({
      ...node,
      style: {
        ...node.style,
        opacity: connectedNodeIds.has(node.id) ? 1 : 0.15,
        transition: 'opacity 0.15s ease'
      }
    })))

    // 更新边样式：关联边保持原样，其他淡化
    setRfEdges(baseEdges.map(edge => ({
      ...edge,
      style: {
        ...edge.style,
        opacity: connectedEdgeIds.has(edge.id)
          ? (edge.style?.opacity || 1)  // 保持原有透明度
          : 0.05,
        transition: 'opacity 0.15s ease'
      },
      labelStyle: {
        ...edge.labelStyle,
        opacity: connectedEdgeIds.has(edge.id) ? 1 : 0
      }
    })))
  }, [hoveredNodeId, setRfNodes, setRfEdges])

  const hasGraph = rfNodes.length > 0
  const hasSelection = activeFile || selectedDirSummary

  return (
    <div className="h-full flex flex-col bg-[#1e1e1e]">
      {/* Status Bar */}
      <div className="h-7 flex items-center justify-between px-3 bg-[#252526] border-b border-[#3c3c3c] shrink-0">
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
        {/* 图例 */}
        {hasGraph && (
          <div className="flex items-center gap-3 text-[10px]">
            <span className="flex items-center gap-1">
              <span className="w-3 h-0.5 bg-blue-500"></span>
              <span className="text-gray-500">imports</span>
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-0.5 bg-green-500" style={{ borderTop: '2px dashed' }}></span>
              <span className="text-gray-500">calls</span>
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-0.5 bg-purple-500"></span>
              <span className="text-gray-500">inherits</span>
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-0.5 bg-gray-500 opacity-50"></span>
              <span className="text-gray-500">contains</span>
            </span>
          </div>
        )}
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
            onNodeMouseEnter={onNodeMouseEnter}
            onNodeMouseLeave={onNodeMouseLeave}
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
  nodeWidth: 180,
  nodeHeight: 50,
  rankSep: 100,     // 层间距离（增大以便区分层级）
  nodeSep: 60,      // 同层节点间距
  edgeSep: 30,      // 边间距
  rankDir: 'TB'     // 布局方向: TB(上到下), LR(左到右)
}

// 节点类型层级映射（数值越小越靠上）
const NODE_TYPE_RANK: Record<string, number> = {
  file: 0,
  module: 0,
  directory: 0,
  class: 1,
  interface: 1,
  struct: 1,
  enum: 1,
  function: 2,
  method: 3,
  constant: 2
}

// 边类型权重（权重越高，节点越靠近）
const EDGE_WEIGHTS: Record<string, number> = {
  contains: 10,     // 包含关系最强，强制父子垂直排列
  inherits: 5,      // 继承关系较强
  implements: 5,    // 实现关系较强
  imports: 2,       // 导入关系中等
  calls: 1,         // 调用关系最弱
  depends: 1
}

/**
 * 使用 Dagre 进行图布局
 * 参考: https://reactflow.dev/examples/layout/dagre
 *
 * 优化策略：
 * 1. 根据节点类型分配层级约束，让同类型节点在同一水平线
 * 2. 根据边类型设置权重，contains关系节点垂直紧密排列
 * 3. 使用 minlen 控制边的最小跨层数
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
    marginy: 50,
    ranker: 'tight-tree'  // 使用 tight-tree 算法减少边交叉
  })

  // 按类型分组节点，用于层级约束
  const nodesByRank = new Map<number, string[]>()

  // 添加节点到 dagre 图
  for (const node of nodes) {
    const rank = NODE_TYPE_RANK[node.type] ?? 2
    g.setNode(node.id, {
      width: nodeWidth,
      height: nodeHeight,
      label: node.label
    })

    // 收集同层级节点
    if (!nodesByRank.has(rank)) {
      nodesByRank.set(rank, [])
    }
    nodesByRank.get(rank)!.push(node.id)
  }

  // 添加边到 dagre 图，设置权重和最小长度
  for (const edge of edges) {
    // 检查源和目标节点都存在
    if (g.hasNode(edge.source) && g.hasNode(edge.target)) {
      const weight = EDGE_WEIGHTS[edge.type] ?? 1
      // contains 边强制垂直排列（minlen=1），其他边允许跨层
      const minlen = edge.type === 'contains' ? 1 : 2
      g.setEdge(edge.source, edge.target, {
        weight,
        minlen
      })
    }
  }

  // 为同类型节点添加隐藏边以保持同层对齐
  // 这会让 Dagre 尽量将相同类型的节点放在同一层
  const ranks = Array.from(nodesByRank.keys()).sort((a, b) => a - b)
  for (let i = 0; i < ranks.length - 1; i++) {
    const currentRankNodes = nodesByRank.get(ranks[i])!
    const nextRankNodes = nodesByRank.get(ranks[i + 1])!

    // 在相邻层级之间建立弱连接
    if (currentRankNodes.length > 0 && nextRankNodes.length > 0) {
      // 只在没有直接连接时添加辅助边
      const firstCurrent = currentRankNodes[0]
      const firstNext = nextRankNodes[0]
      if (!g.hasEdge(firstCurrent, firstNext)) {
        // 添加一条非常弱的边来引导层级
        g.setEdge(firstCurrent, firstNext, {
          weight: 0.1,
          minlen: 1,
          style: 'invis'  // 标记为不可见（实际渲染时过滤）
        })
      }
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
