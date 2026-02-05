/**
 * 文档路径映射工具
 *
 * 用于在源码路径和文档路径之间进行转换
 */

// 文档目录后缀
const DOCS_SUFFIX = '_docs'
// 目录总结文件名
const DIR_SUMMARY_NAME = '_dir_summary.md'

/**
 * 根据源码项目路径计算文档目录路径
 * @param sourcePath 源码项目路径
 * @returns 文档目录路径
 */
export function getDocsRootPath(sourcePath: string): string {
  // 标准化路径分隔符
  const normalizedPath = sourcePath.replace(/\\/g, '/')
  const pathParts = normalizedPath.split('/')
  const projectName = pathParts[pathParts.length - 1]
  const parentPath = pathParts.slice(0, -1).join('/')
  return `${parentPath}/${projectName}${DOCS_SUFFIX}`
}

/**
 * 源码文件路径转换为文档路径
 * @param sourcePath 源码文件的完整路径
 * @param sourceRoot 源码根目录
 * @param docsRoot 文档根目录
 * @returns 对应的文档路径
 *
 * 例如: src/main.py -> project_docs/src/main.py.md
 */
export function sourceToDocPath(
  sourcePath: string,
  sourceRoot: string,
  docsRoot: string
): string {
  // 标准化路径分隔符
  const normalizedSourcePath = sourcePath.replace(/\\/g, '/')
  const normalizedSourceRoot = sourceRoot.replace(/\\/g, '/')
  const normalizedDocsRoot = docsRoot.replace(/\\/g, '/')

  // 计算相对路径
  const relativePath = normalizedSourcePath.replace(normalizedSourceRoot, '').replace(/^\//, '')

  // 返回文档路径
  return `${normalizedDocsRoot}/${relativePath}.md`
}

/**
 * 目录路径转换为目录总结文档路径
 * @param dirPath 目录的完整路径
 * @param sourceRoot 源码根目录
 * @param docsRoot 文档根目录
 * @returns 对应的目录总结文档路径
 *
 * 例如: src/utils/ -> project_docs/src/utils/_dir_summary.md
 */
export function getDirSummaryPath(
  dirPath: string,
  sourceRoot: string,
  docsRoot: string
): string {
  // 标准化路径分隔符
  const normalizedDirPath = dirPath.replace(/\\/g, '/')
  const normalizedSourceRoot = sourceRoot.replace(/\\/g, '/')
  const normalizedDocsRoot = docsRoot.replace(/\\/g, '/')

  // 计算相对路径
  const relativePath = normalizedDirPath.replace(normalizedSourceRoot, '').replace(/^\//, '')

  // 如果是根目录
  if (!relativePath || relativePath === '') {
    return `${normalizedDocsRoot}/${DIR_SUMMARY_NAME}`
  }

  // 返回目录总结文档路径
  return `${normalizedDocsRoot}/${relativePath}/${DIR_SUMMARY_NAME}`
}

/**
 * 检查路径是否是文档目录
 * @param path 要检查的路径
 * @returns 是否是文档目录
 */
export function isDocsDirectory(path: string): boolean {
  const normalizedPath = path.replace(/\\/g, '/')
  return normalizedPath.endsWith(DOCS_SUFFIX) || normalizedPath.includes(`${DOCS_SUFFIX}/`)
}

/**
 * 从文档路径提取对应的源码相对路径
 * @param docPath 文档路径
 * @param docsRoot 文档根目录
 * @returns 源码相对路径
 */
export function docToSourceRelativePath(docPath: string, docsRoot: string): string | null {
  const normalizedDocPath = docPath.replace(/\\/g, '/')
  const normalizedDocsRoot = docsRoot.replace(/\\/g, '/')

  if (!normalizedDocPath.startsWith(normalizedDocsRoot)) {
    return null
  }

  let relativePath = normalizedDocPath.replace(normalizedDocsRoot, '').replace(/^\//, '')

  // 移除 .md 后缀（如果是文件文档）
  if (relativePath.endsWith('.md') && !relativePath.endsWith(DIR_SUMMARY_NAME)) {
    relativePath = relativePath.slice(0, -3)
  }

  // 移除目录总结文件名
  if (relativePath.endsWith(DIR_SUMMARY_NAME)) {
    relativePath = relativePath.replace(DIR_SUMMARY_NAME, '').replace(/\/$/, '')
  }

  return relativePath
}

/**
 * 获取文件对应的文档文件名
 * @param fileName 源文件名
 * @returns 文档文件名
 */
export function getDocFileName(fileName: string): string {
  return `${fileName}.md`
}

export { DOCS_SUFFIX, DIR_SUMMARY_NAME }
