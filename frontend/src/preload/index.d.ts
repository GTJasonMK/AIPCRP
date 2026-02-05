import { ElectronAPI } from '@electron-toolkit/preload'

interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

interface FileInfo {
  content: string
  extension: string
  path: string
}

interface DirectoryEntry {
  name: string
  path: string
  type: 'file' | 'directory'
}

interface RecentProject {
  path: string
  name: string
  lastOpened: number
}

interface Api {
  selectDirectory: () => Promise<string | null>
  readDirectory: (dirPath: string) => Promise<DirectoryEntry[]>
  readFile: (filePath: string) => Promise<FileInfo>
  getFileTree: (rootPath: string) => Promise<FileTreeNode[]>
  getBackendPort: () => Promise<number>
  getRecentProjects: () => Promise<RecentProject[]>
  addRecentProject: (projectPath: string) => Promise<RecentProject[]>
  removeRecentProject: (projectPath: string) => Promise<RecentProject[]>
  platform: NodeJS.Platform
}

declare global {
  interface Window {
    electron: ElectronAPI
    api: Api
  }
}
