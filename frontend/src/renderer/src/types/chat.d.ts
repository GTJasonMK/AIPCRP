export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

export interface ChatContext {
  projectPath?: string
  currentFile?: string
  currentFileContent?: string
  selectedCode?: string
  fileTreeSummary?: string
}

export type WebSocketStatus = 'connecting' | 'connected' | 'disconnected'
