import { create } from 'zustand'

interface FileInfo {
  path: string
  name: string
  content: string
  extension: string
}

interface DirSummary {
  name: string
  path: string
  content: string
}

interface EditorStore {
  // State
  openFiles: FileInfo[]
  activeFile: FileInfo | null
  selectedCode: string

  // Doc-related state
  activeDocContent: string | null
  selectedDirSummary: DirSummary | null
  docsBasePath: string | null
  projectPath: string | null  // 项目根路径

  // Actions
  openFile: (path: string) => Promise<void>
  closeFile: (path: string) => void
  setActiveFile: (path: string) => void
  setSelectedCode: (code: string) => void
  setDocsBasePath: (path: string | null) => void
  setProjectPath: (path: string | null) => void
  loadDocForFile: (filePath: string) => Promise<void>
  loadDirSummary: (dirPath: string, dirName: string) => Promise<void>
  clearDocContent: () => void
  clearDirSummary: () => void
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  openFiles: [],
  activeFile: null,
  selectedCode: '',
  activeDocContent: null,
  selectedDirSummary: null,
  docsBasePath: null,
  projectPath: null,

  openFile: async (filePath: string) => {
    const { openFiles, docsBasePath } = get()

    // Check if file is already open
    const existingFile = openFiles.find(f => f.path === filePath)
    if (existingFile) {
      set({ activeFile: existingFile, selectedDirSummary: null })
      // Try to load corresponding doc
      if (docsBasePath) {
        get().loadDocForFile(filePath)
      }
      return
    }

    try {
      const fileInfo = await window.api.readFile(filePath)
      const fileName = filePath.split(/[\\/]/).pop() || filePath

      const newFile: FileInfo = {
        path: filePath,
        name: fileName,
        content: fileInfo.content,
        extension: fileInfo.extension
      }

      set({
        openFiles: [...openFiles, newFile],
        activeFile: newFile,
        selectedDirSummary: null
      })

      // Try to load corresponding doc
      if (docsBasePath) {
        get().loadDocForFile(filePath)
      }
    } catch (error) {
      console.error('Failed to open file:', error)
    }
  },

  closeFile: (filePath: string) => {
    const { openFiles, activeFile } = get()
    const newOpenFiles = openFiles.filter(f => f.path !== filePath)

    let newActiveFile = activeFile
    if (activeFile?.path === filePath) {
      newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null
    }

    set({
      openFiles: newOpenFiles,
      activeFile: newActiveFile
    })

    // Clear doc if no active file
    if (!newActiveFile) {
      set({ activeDocContent: null })
    } else if (get().docsBasePath) {
      // Load doc for the new active file
      get().loadDocForFile(newActiveFile.path)
    }
  },

  setActiveFile: (filePath: string) => {
    const { openFiles, docsBasePath } = get()
    const file = openFiles.find(f => f.path === filePath)
    if (file) {
      set({ activeFile: file, selectedDirSummary: null })
      // Try to load corresponding doc
      if (docsBasePath) {
        get().loadDocForFile(filePath)
      }
    }
  },

  setSelectedCode: (code: string) => set({ selectedCode: code }),

  setProjectPath: (path: string | null) => set({ projectPath: path }),

  setDocsBasePath: (path: string | null) => {
    set({ docsBasePath: path })
    // Reload doc for current file if we have one
    const { activeFile } = get()
    if (path && activeFile) {
      get().loadDocForFile(activeFile.path)
    }
  },

  loadDocForFile: async (filePath: string) => {
    const { docsBasePath, projectPath } = get()
    if (!docsBasePath) {
      set({ activeDocContent: null })
      return
    }

    try {
      // 计算相对路径
      // 源码路径: E:\project\src\file.ts
      // 项目路径: E:\project
      // 文档基路径: E:\project\project_docs
      // 期望文档路径: E:\project\project_docs\src\file.ts.md

      let relativePath: string

      if (projectPath) {
        // 使用项目路径计算相对路径
        relativePath = filePath
          .replace(projectPath, '')
          .replace(/^[\\/]/, '')
          .replace(/\\/g, '/')
      } else {
        // 回退：使用 docsBasePath 的父目录
        const docsParent = docsBasePath.replace(/[\\/][^\\/]+$/, '')
        relativePath = filePath
          .replace(docsParent, '')
          .replace(/^[\\/]/, '')
          .replace(/\\/g, '/')
      }

      const docPath = `${docsBasePath}/${relativePath}.md`.replace(/\\/g, '/')
      console.log('[loadDocForFile] Looking for doc at:', docPath)

      const docInfo = await window.api.readFile(docPath)
      set({ activeDocContent: docInfo.content })
    } catch {
      // No doc file exists for this source file
      set({ activeDocContent: null })
    }
  },

  loadDirSummary: async (dirPath: string, dirName: string) => {
    const { docsBasePath, projectPath } = get()
    if (!docsBasePath) {
      set({ selectedDirSummary: null })
      return
    }

    try {
      // 计算目录总结路径
      let relativePath: string

      if (projectPath) {
        relativePath = dirPath
          .replace(projectPath, '')
          .replace(/^[\\/]/, '')
          .replace(/\\/g, '/')
      } else {
        const docsParent = docsBasePath.replace(/[\\/][^\\/]+$/, '')
        relativePath = dirPath
          .replace(docsParent, '')
          .replace(/^[\\/]/, '')
          .replace(/\\/g, '/')
      }

      // 目录总结使用 _dir_summary.md
      const summaryPath = relativePath
        ? `${docsBasePath}/${relativePath}/_dir_summary.md`.replace(/\\/g, '/')
        : `${docsBasePath}/_dir_summary.md`.replace(/\\/g, '/')

      console.log('[loadDirSummary] Looking for summary at:', summaryPath)

      const docInfo = await window.api.readFile(summaryPath)
      set({
        selectedDirSummary: {
          name: dirName,
          path: dirPath,
          content: docInfo.content
        },
        activeDocContent: null
      })
    } catch {
      set({ selectedDirSummary: null })
    }
  },

  clearDocContent: () => set({
    activeDocContent: null,
    selectedDirSummary: null
  }),

  clearDirSummary: () => set({ selectedDirSummary: null })
}))
