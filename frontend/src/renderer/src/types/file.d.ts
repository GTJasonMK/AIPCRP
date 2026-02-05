export interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
}

export interface FileInfo {
  content: string
  extension: string
  path: string
}

export interface DirectoryEntry {
  name: string
  path: string
  type: 'file' | 'directory'
}
