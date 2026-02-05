import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

// 文件树节点类型
interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

// 文件信息类型
interface FileInfo {
  content: string
  extension: string
  path: string
}

// 目录条目类型
interface DirectoryEntry {
  name: string
  path: string
  type: 'file' | 'directory'
}

// 最近项目类型
interface RecentProject {
  path: string
  name: string
  lastOpened: number
}

// 自定义API
const api = {
  // 文件系统操作
  selectDirectory: (): Promise<string | null> =>
    ipcRenderer.invoke('dialog:selectDirectory'),

  readDirectory: (dirPath: string): Promise<DirectoryEntry[]> =>
    ipcRenderer.invoke('fs:readDirectory', dirPath),

  readFile: (filePath: string): Promise<FileInfo> =>
    ipcRenderer.invoke('fs:readFile', filePath),

  getFileTree: (rootPath: string): Promise<FileTreeNode[]> =>
    ipcRenderer.invoke('fs:getFileTree', rootPath),

  // 应用信息
  getBackendPort: (): Promise<number> =>
    ipcRenderer.invoke('app:getBackendPort'),

  // 最近项目
  getRecentProjects: (): Promise<RecentProject[]> =>
    ipcRenderer.invoke('app:getRecentProjects'),

  addRecentProject: (projectPath: string): Promise<RecentProject[]> =>
    ipcRenderer.invoke('app:addRecentProject', projectPath),

  removeRecentProject: (projectPath: string): Promise<RecentProject[]> =>
    ipcRenderer.invoke('app:removeRecentProject', projectPath),

  // 平台信息
  platform: process.platform
}

// 类型声明
declare global {
  interface Window {
    electron: typeof electronAPI
    api: typeof api
  }
}

// 使用contextBridge安全暴露API
if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', api)
  } catch (error) {
    console.error(error)
  }
} else {
  window.electron = electronAPI
  window.api = api
}
