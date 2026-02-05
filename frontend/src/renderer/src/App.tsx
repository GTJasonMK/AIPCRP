import { useEffect } from 'react'
import { MainLayout } from './components/layout/MainLayout'
import { useChatStore } from './stores/chatStore'

function App(): JSX.Element {
  const { connect } = useChatStore()

  useEffect(() => {
    // Get backend port and connect WebSocket
    const initWebSocket = async () => {
      try {
        const port = await window.api.getBackendPort()
        // Wait for backend startup
        setTimeout(() => {
          connect(port)
        }, 2000)
      } catch (error) {
        console.error('Failed to get backend port:', error)
      }
    }

    initWebSocket()
  }, [connect])

  return <MainLayout />
}

export default App
