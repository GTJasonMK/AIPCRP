import { useState, useEffect } from 'react'
import { useChatStore } from '../../stores/chatStore'

interface AppConfig {
  api_key_set: boolean
  base_url: string
  model: string
  temperature: number
  max_tokens: number
}

interface SettingsModalProps {
  onClose: () => void
}

export function SettingsModal({ onClose }: SettingsModalProps): JSX.Element {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [apiKey, setApiKey] = useState('')
  const [baseUrl, setBaseUrl] = useState('')
  const [model, setModel] = useState('')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [message, setMessage] = useState('')
  const [messageType, setMessageType] = useState<'success' | 'error' | 'info'>('info')

  const { checkApiConfig } = useChatStore()

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      const port = await window.api.getBackendPort()
      const response = await fetch(`http://127.0.0.1:${port}/api/config`)
      if (response.ok) {
        const data = await response.json()
        setConfig(data)
        setBaseUrl(data.base_url)
        setModel(data.model)
      }
    } catch (error) {
      console.error('Failed to load config:', error)
      setMessage('Failed to load config')
      setMessageType('error')
    } finally {
      setLoading(false)
    }
  }

  const testConnection = async () => {
    setTesting(true)
    setMessage('')

    // Check if we have enough info to test
    const hasApiKey = apiKey || config?.api_key_set
    if (!hasApiKey) {
      setMessage('Please enter an API Key first')
      setMessageType('error')
      setTesting(false)
      return
    }

    try {
      const port = await window.api.getBackendPort()
      const body: Record<string, string> = {}

      // Send current form values for testing
      if (apiKey) body.api_key = apiKey
      if (baseUrl) body.base_url = baseUrl
      if (model) body.model = model

      const response = await fetch(`http://127.0.0.1:${port}/api/config/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      })

      const data = await response.json()

      if (response.ok) {
        setMessage(`Connection successful! Model: ${data.model}`)
        setMessageType('success')
      } else {
        setMessage(data.detail || 'Connection failed')
        setMessageType('error')
      }
    } catch (error) {
      console.error('Test connection failed:', error)
      setMessage('Test failed: Network error')
      setMessageType('error')
    } finally {
      setTesting(false)
    }
  }

  const saveConfig = async () => {
    setSaving(true)
    setMessage('')

    try {
      const port = await window.api.getBackendPort()
      const body: Record<string, string> = {}

      if (apiKey) body.api_key = apiKey
      if (baseUrl) body.base_url = baseUrl
      if (model) body.model = model

      const response = await fetch(`http://127.0.0.1:${port}/api/config`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      })

      if (response.ok) {
        setMessage('Config saved')
        setMessageType('success')
        setApiKey('')
        // Refresh API config status
        await checkApiConfig(port)
        setTimeout(() => {
          onClose()
        }, 1000)
      } else {
        setMessage('Failed to save')
        setMessageType('error')
      }
    } catch (error) {
      console.error('Failed to save config:', error)
      setMessage('Failed to save')
      setMessageType('error')
    } finally {
      setSaving(false)
    }
  }

  const getMessageColor = () => {
    switch (messageType) {
      case 'success': return 'text-green-500'
      case 'error': return 'text-red-500'
      default: return 'text-gray-400'
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-[#252526] rounded-lg shadow-xl w-[500px] max-h-[80vh] overflow-auto">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[#3c3c3c]">
          <h2 className="text-lg font-medium text-white">Settings</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {loading ? (
            <div className="text-center text-gray-400 py-8">Loading...</div>
          ) : (
            <>
              {/* API Key */}
              <div>
                <label className="block text-sm text-gray-300 mb-2">
                  API Key
                  {config?.api_key_set && (
                    <span className="ml-2 text-green-500 text-xs">(Configured)</span>
                  )}
                </label>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={config?.api_key_set ? 'Leave empty to keep current' : 'Enter API Key'}
                  className="w-full bg-[#3c3c3c] text-white text-sm rounded px-3 py-2 focus:outline-none focus:ring-1 focus:ring-[#0e639c]"
                />
              </div>

              {/* Base URL */}
              <div>
                <label className="block text-sm text-gray-300 mb-2">API Base URL</label>
                <input
                  type="text"
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.target.value)}
                  placeholder="https://api.openai.com"
                  className="w-full bg-[#3c3c3c] text-white text-sm rounded px-3 py-2 focus:outline-none focus:ring-1 focus:ring-[#0e639c]"
                />
                <p className="text-xs text-gray-500 mt-1">Supports OpenAI-compatible endpoints</p>
              </div>

              {/* Model */}
              <div>
                <label className="block text-sm text-gray-300 mb-2">Model</label>
                <input
                  type="text"
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  placeholder="gpt-4o"
                  className="w-full bg-[#3c3c3c] text-white text-sm rounded px-3 py-2 focus:outline-none focus:ring-1 focus:ring-[#0e639c]"
                />
                <p className="text-xs text-gray-500 mt-1">
                  Supports: gpt-4o, gpt-4-turbo, claude-3-5-sonnet, etc.
                </p>
              </div>

              {/* Test Connection Button */}
              <div>
                <button
                  onClick={testConnection}
                  disabled={testing || loading}
                  className="px-4 py-2 text-sm bg-[#2d2d2d] hover:bg-[#3c3c3c] text-gray-300 rounded border border-[#3c3c3c] disabled:opacity-50 flex items-center gap-2"
                >
                  {testing ? (
                    <>
                      <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                      </svg>
                      Testing...
                    </>
                  ) : (
                    <>
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                      </svg>
                      Test Connection
                    </>
                  )}
                </button>
              </div>

              {/* Message */}
              {message && (
                <p className={`text-sm ${getMessageColor()}`}>
                  {message}
                </p>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 px-6 py-4 border-t border-[#3c3c3c]">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-300 hover:text-white"
          >
            Cancel
          </button>
          <button
            onClick={saveConfig}
            disabled={saving || loading}
            className="px-4 py-2 text-sm bg-[#0e639c] hover:bg-[#1177bb] text-white rounded disabled:opacity-50"
          >
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  )
}
