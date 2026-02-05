/**
 * 文档生成 API 服务
 *
 * 封装与后端文档生成 API 的通信
 */

// API 响应类型
interface GenerateDocsResponse {
  task_id: string
  docs_path: string
}

interface TaskStatusResponse {
  id: string
  status: string
  progress: number
  current_file: string | null
  stats: {
    total_files: number
    processed_files: number
    total_dirs: number
    processed_dirs: number
    failed_count: number
    skipped_count: number
  }
  error: string | null
}

// WebSocket 消息类型（后端使用 snake_case）
interface WsProgressMessage {
  type: 'progress'
  progress: number
  current_file: string | null
  stats: TaskStatusResponse['stats']
}

interface WsFileStartedMessage {
  type: 'file_started'
  path: string
}

interface WsFileCompletedMessage {
  type: 'file_completed'
  path: string
}

interface WsDirStartedMessage {
  type: 'dir_started'
  path: string
}

interface WsDirCompletedMessage {
  type: 'dir_completed'
  path: string
}

interface WsCompletedMessage {
  type: 'completed'
  stats: TaskStatusResponse['stats']
}

interface WsErrorMessage {
  type: 'error'
  message: string
}

interface WsCancelledMessage {
  type: 'cancelled'
}

type WsDocMessage =
  | WsProgressMessage
  | WsFileStartedMessage
  | WsFileCompletedMessage
  | WsDirStartedMessage
  | WsDirCompletedMessage
  | WsCompletedMessage
  | WsErrorMessage
  | WsCancelledMessage

// 进度回调
interface ProgressCallbacks {
  onProgress?: (progress: number, currentFile: string | null, stats: TaskStatusResponse['stats']) => void
  onFileStarted?: (path: string) => void
  onFileCompleted?: (path: string) => void
  onDirStarted?: (path: string) => void
  onDirCompleted?: (path: string) => void
  onCompleted?: (stats: TaskStatusResponse['stats']) => void
  onError?: (message: string) => void
  onCancelled?: () => void
}

let backendPort = 8765 // 默认端口

/**
 * 设置后端端口号
 */
export function setBackendPort(port: number): void {
  backendPort = port
}

/**
 * 获取 API 基础 URL
 */
function getBaseUrl(): string {
  return `http://127.0.0.1:${backendPort}`
}

/**
 * 启动文档生成任务
 */
export async function startDocGeneration(
  sourcePath: string,
  docsPath?: string,
  resume?: boolean
): Promise<GenerateDocsResponse> {
  const response = await fetch(`${getBaseUrl()}/api/docs/generate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      source_path: sourcePath,
      docs_path: docsPath,
      resume: resume ?? true
    })
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || '启动文档生成失败')
  }

  return response.json()
}

/**
 * 获取任务状态
 */
export async function getTaskStatus(taskId: string): Promise<TaskStatusResponse> {
  const response = await fetch(`${getBaseUrl()}/api/docs/tasks/${taskId}`)

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || '获取任务状态失败')
  }

  return response.json()
}

/**
 * 取消任务
 */
export async function cancelTask(taskId: string): Promise<void> {
  const response = await fetch(`${getBaseUrl()}/api/docs/tasks/${taskId}/cancel`, {
    method: 'POST'
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || '取消任务失败')
  }
}

/**
 * 订阅任务进度（WebSocket）
 * @returns 取消订阅函数
 */
export function subscribeProgress(
  taskId: string,
  callbacks: ProgressCallbacks
): () => void {
  const wsUrl = `ws://127.0.0.1:${backendPort}/ws/docs/${taskId}`
  console.log('[subscribeProgress] Connecting to WebSocket:', wsUrl)

  const ws = new WebSocket(wsUrl)

  ws.onopen = () => {
    console.log('[subscribeProgress] WebSocket connected')
  }

  ws.onmessage = (event) => {
    try {
      console.log('[subscribeProgress] Received message:', event.data)
      const msg = JSON.parse(event.data) as WsDocMessage

      switch (msg.type) {
        case 'progress':
          callbacks.onProgress?.(msg.progress, msg.current_file, msg.stats)
          break
        case 'file_started':
          callbacks.onFileStarted?.(msg.path)
          break
        case 'file_completed':
          callbacks.onFileCompleted?.(msg.path)
          break
        case 'dir_started':
          callbacks.onDirStarted?.(msg.path)
          break
        case 'dir_completed':
          callbacks.onDirCompleted?.(msg.path)
          break
        case 'completed':
          callbacks.onCompleted?.(msg.stats)
          break
        case 'error':
          callbacks.onError?.(msg.message)
          break
        case 'cancelled':
          callbacks.onCancelled?.()
          break
      }
    } catch (error) {
      console.error('解析 WebSocket 消息失败:', error)
    }
  }

  ws.onerror = (error) => {
    console.error('WebSocket 连接错误:', error)
    callbacks.onError?.('WebSocket 连接错误')
  }

  ws.onclose = () => {
    console.log('WebSocket 连接关闭')
  }

  // 返回取消函数
  return () => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close()
    }
  }
}
