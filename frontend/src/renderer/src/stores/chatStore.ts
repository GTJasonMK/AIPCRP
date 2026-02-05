import { create } from 'zustand'

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

interface ChatContext {
  projectPath?: string
  currentFile?: string
  currentFileContent?: string
  selectedCode?: string
  fileTreeSummary?: string
}

type WebSocketStatus = 'connecting' | 'connected' | 'disconnected'

interface ChatStore {
  // State
  messages: ChatMessage[]
  isStreaming: boolean
  websocket: WebSocket | null
  wsStatus: WebSocketStatus
  apiConfigured: boolean
  conversationId: string

  // Actions
  connect: (port: number) => void
  disconnect: () => void
  sendMessage: (content: string, context: ChatContext) => void
  clearMessages: () => void
  checkApiConfig: (port: number) => Promise<void>
}

function generateId(): string {
  return Math.random().toString(36).substring(2, 10)
}

export const useChatStore = create<ChatStore>((set, get) => ({
  messages: [],
  isStreaming: false,
  websocket: null,
  wsStatus: 'disconnected',
  apiConfigured: false,
  conversationId: generateId(),

  checkApiConfig: async (port: number) => {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/api/config`)
      if (res.ok) {
        const data = await res.json()
        set({ apiConfigured: data.api_key_set === true })
      }
    } catch {
      set({ apiConfigured: false })
    }
  },

  connect: (port: number) => {
    const { websocket } = get()
    if (websocket) {
      websocket.close()
    }

    set({ wsStatus: 'connecting' })

    const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/chat`)

    ws.onopen = () => {
      console.log('WebSocket connected')
      set({ wsStatus: 'connected' })
      get().checkApiConfig(port)
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)

        if (data.type === 'chat_chunk') {
          set(state => {
            const messages = [...state.messages]
            const lastMsg = messages[messages.length - 1]
            if (lastMsg && lastMsg.role === 'assistant') {
              lastMsg.content += data.content
            }
            return { messages }
          })
        } else if (data.type === 'chat_done') {
          set({ isStreaming: false })
        } else if (data.type === 'chat_error') {
          set(state => {
            const messages = [...state.messages]
            const lastMsg = messages[messages.length - 1]
            if (lastMsg && lastMsg.role === 'assistant') {
              lastMsg.content = `Error: ${data.error}`
            }
            return { messages, isStreaming: false }
          })
        } else if (data.type === 'pong') {
          // Heartbeat response
        }
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error)
      }
    }

    ws.onerror = (error) => {
      console.error('WebSocket error:', error)
      set({ wsStatus: 'disconnected', isStreaming: false })
    }

    ws.onclose = () => {
      console.log('WebSocket disconnected')
      set({ wsStatus: 'disconnected', isStreaming: false })

      // Auto reconnect after 3 seconds
      setTimeout(() => {
        const { wsStatus } = get()
        if (wsStatus === 'disconnected') {
          get().connect(port)
        }
      }, 3000)
    }

    set({ websocket: ws })

    // Heartbeat
    const heartbeat = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'ping' }))
      } else {
        clearInterval(heartbeat)
      }
    }, 30000)
  },

  disconnect: () => {
    const { websocket } = get()
    if (websocket) {
      websocket.close()
    }
    set({ websocket: null, wsStatus: 'disconnected' })
  },

  sendMessage: (content: string, context: ChatContext) => {
    const { websocket, conversationId, isStreaming } = get()

    if (!websocket || websocket.readyState !== WebSocket.OPEN) {
      console.error('WebSocket not connected')
      return
    }

    if (isStreaming) {
      return
    }

    // Add user message
    const userMsg: ChatMessage = {
      id: generateId(),
      role: 'user',
      content,
      timestamp: Date.now()
    }

    // Add empty assistant message
    const assistantMsg: ChatMessage = {
      id: generateId(),
      role: 'assistant',
      content: '',
      timestamp: Date.now()
    }

    set(state => ({
      messages: [...state.messages, userMsg, assistantMsg],
      isStreaming: true
    }))

    websocket.send(JSON.stringify({
      type: 'chat_message',
      content,
      context,
      conversationId
    }))
  },

  clearMessages: () => set({
    messages: [],
    conversationId: generateId()
  })
}))
