import { app, shell, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import { spawn, ChildProcess, execSync } from 'child_process'
import * as fs from 'fs/promises'
import * as fsSync from 'fs'
import * as path from 'path'

let mainWindow: BrowserWindow | null = null
let backendProcess: ChildProcess | null = null
const BACKEND_PORT = 8765
const MAX_RECENT_PROJECTS = 10

// 最近打开的项目
interface RecentProject {
  path: string
  name: string
  lastOpened: number // Unix 时间戳（毫秒）
}

// 获取最近项目列表的存储路径
function getRecentProjectsPath(): string {
  return path.join(app.getPath('userData'), 'recent-projects.json')
}

// 读取最近项目列表
async function loadRecentProjects(): Promise<RecentProject[]> {
  try {
    const filePath = getRecentProjectsPath()
    const content = await fs.readFile(filePath, 'utf-8')
    const data = JSON.parse(content)
    if (Array.isArray(data)) {
      return data
    }
    return []
  } catch {
    return []
  }
}

// 保存最近项目列表
async function saveRecentProjects(projects: RecentProject[]): Promise<void> {
  const filePath = getRecentProjectsPath()
  await fs.writeFile(filePath, JSON.stringify(projects, null, 2), 'utf-8')
}

// 添加或更新最近项目
async function addRecentProject(projectPath: string): Promise<RecentProject[]> {
  const projects = await loadRecentProjects()
  const name = path.basename(projectPath)
  const now = Date.now()

  // 移除已有的相同路径（如果存在）
  const filtered = projects.filter(p => p.path !== projectPath)

  // 添加到列表头部
  filtered.unshift({ path: projectPath, name, lastOpened: now })

  // 保留最近 N 个
  const trimmed = filtered.slice(0, MAX_RECENT_PROJECTS)

  await saveRecentProjects(trimmed)
  return trimmed
}

// Directories to filter out
const IGNORED_DIRS = ['.git', 'node_modules', '__pycache__', '.venv', 'venv',
  'dist', 'build', '.idea', '.vscode', '.next', 'out', '.cache']

interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    show: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow?.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

function killPortProcess(port: number): void {
  try {
    // Find and kill any process occupying the port (Windows)
    const result = execSync(
      `netstat -ano | findstr :${port} | findstr LISTENING`,
      { encoding: 'utf-8', timeout: 5000 }
    )
    const lines = result.trim().split('\n')
    const pids = new Set<string>()
    for (const line of lines) {
      const parts = line.trim().split(/\s+/)
      const pid = parts[parts.length - 1]
      if (pid && pid !== '0') pids.add(pid)
    }
    for (const pid of pids) {
      try {
        execSync(`taskkill /pid ${pid} /f /t`, { timeout: 5000 })
        console.log(`Killed process ${pid} on port ${port}`)
      } catch { /* ignore */ }
    }
  } catch {
    // No LISTENING process on this port, safe to proceed
  }
}

function startBackend(): void {
  // Kill any lingering process on the backend port
  killPortProcess(BACKEND_PORT)

  let command: string
  let args: string[]
  let cwd: string

  if (is.dev) {
    // Dev mode: use Rust backend
    const projectRoot = join(__dirname, '../../..')
    const rustReleaseExe = join(projectRoot, 'backend-rs/target/release/backend-rs.exe')
    const rustDebugExe = join(projectRoot, 'backend-rs/target/debug/backend-rs.exe')

    if (fsSync.existsSync(rustReleaseExe)) {
      // Use Rust release build
      command = rustReleaseExe
      args = []
      cwd = join(projectRoot, 'backend-rs')
      console.log('Dev mode - Using Rust backend (release):', rustReleaseExe)
    } else if (fsSync.existsSync(rustDebugExe)) {
      // Use Rust debug build
      command = rustDebugExe
      args = []
      cwd = join(projectRoot, 'backend-rs')
      console.log('Dev mode - Using Rust backend (debug):', rustDebugExe)
    } else {
      console.error('Rust backend not found. Please run: cd backend-rs && cargo build --release')
      return
    }
  } else {
    // Production mode: use bundled Rust backend
    const backendDir = join(process.resourcesPath, 'backend')
    const backendExe = join(backendDir, 'backend-rs.exe')

    command = backendExe
    args = []
    cwd = backendDir

    console.log('Production mode - Backend exe:', backendExe)
  }

  backendProcess = spawn(command, args, {
    cwd: cwd,
    env: { ...process.env },
    stdio: ['pipe', 'pipe', 'pipe']
  })

  backendProcess.stdout?.on('data', (data) => {
    console.log(`Backend: ${data}`)
  })

  backendProcess.stderr?.on('data', (data) => {
    console.error(`Backend: ${data}`)
  })

  backendProcess.on('error', (error) => {
    console.error('Backend process failed to start:', error)
  })

  backendProcess.on('exit', (code) => {
    console.log('Backend process exited, code:', code)
  })
}

function registerIpcHandlers(): void {
  // Select directory dialog
  ipcMain.handle('dialog:selectDirectory', async () => {
    const result = await dialog.showOpenDialog({
      properties: ['openDirectory']
    })
    return result.canceled ? null : result.filePaths[0]
  })

  // Read directory entries (single level)
  ipcMain.handle('fs:readDirectory', async (_, dirPath: string) => {
    try {
      const entries = await fs.readdir(dirPath, { withFileTypes: true })
      return entries
        .filter(e => !IGNORED_DIRS.includes(e.name) && !e.name.startsWith('.'))
        .map(e => ({
          name: e.name,
          path: path.join(dirPath, e.name),
          type: e.isDirectory() ? 'directory' : 'file'
        }))
        .sort((a, b) => {
          if (a.type !== b.type) return a.type === 'directory' ? -1 : 1
          return a.name.localeCompare(b.name)
        })
    } catch (error) {
      console.error('Failed to read directory:', error)
      return []
    }
  })

  // Read file content
  ipcMain.handle('fs:readFile', async (_, filePath: string) => {
    try {
      const content = await fs.readFile(filePath, 'utf-8')
      const ext = path.extname(filePath).slice(1)
      return { content, extension: ext, path: filePath }
    } catch (error) {
      console.error('Failed to read file:', error)
      throw error
    }
  })

  // Get full file tree (recursive)
  ipcMain.handle('fs:getFileTree', async (_, rootPath: string) => {
    async function buildTree(dirPath: string, depth = 0): Promise<FileTreeNode[]> {
      // 增加深度限制到10层，与后端保持一致
      if (depth > 10) return []

      try {
        const entries = await fs.readdir(dirPath, { withFileTypes: true })
        const nodes: FileTreeNode[] = []

        for (const entry of entries) {
          // 跳过忽略的目录
          if (IGNORED_DIRS.includes(entry.name)) continue
          // 跳过隐藏文件/目录，但保留 .docs 目录（用于检测已有文档）
          if (entry.name.startsWith('.') && entry.name !== '.docs') continue

          const fullPath = path.join(dirPath, entry.name)
          if (entry.isDirectory()) {
            nodes.push({
              name: entry.name,
              path: fullPath,
              type: 'directory',
              children: await buildTree(fullPath, depth + 1)
            })
          } else {
            nodes.push({
              name: entry.name,
              path: fullPath,
              type: 'file'
            })
          }
        }

        return nodes.sort((a, b) => {
          if (a.type !== b.type) return a.type === 'directory' ? -1 : 1
          return a.name.localeCompare(b.name)
        })
      } catch {
        return []
      }
    }

    return buildTree(rootPath)
  })

  // Get backend port
  ipcMain.handle('app:getBackendPort', () => BACKEND_PORT)

  // 最近项目相关
  ipcMain.handle('app:getRecentProjects', async () => {
    return loadRecentProjects()
  })

  ipcMain.handle('app:addRecentProject', async (_, projectPath: string) => {
    return addRecentProject(projectPath)
  })

  ipcMain.handle('app:removeRecentProject', async (_, projectPath: string) => {
    const projects = await loadRecentProjects()
    const filtered = projects.filter(p => p.path !== projectPath)
    await saveRecentProjects(filtered)
    return filtered
  })
}

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.ai-code-review')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  startBackend()
  registerIpcHandlers()
  createWindow()

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

function killBackend(): void {
  if (backendProcess) {
    const pid = backendProcess.pid
    if (pid) {
      try {
        // On Windows, use taskkill to force kill the process tree
        if (process.platform === 'win32') {
          spawn('taskkill', ['/pid', String(pid), '/f', '/t'], { detached: true })
        } else {
          process.kill(-pid, 'SIGKILL')
        }
      } catch (e) {
        console.error('Failed to kill backend process:', e)
      }
    }
    backendProcess.kill('SIGKILL')
    backendProcess = null
  }
}

app.on('before-quit', () => {
  killBackend()
})

app.on('quit', () => {
  killBackend()
})

// Handle unexpected exits
process.on('exit', () => {
  killBackend()
})

process.on('SIGINT', () => {
  killBackend()
  process.exit(0)
})

process.on('SIGTERM', () => {
  killBackend()
  process.exit(0)
})
