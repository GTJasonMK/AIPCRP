import { memo } from 'react'
import { Handle, Position, NodeProps } from 'reactflow'

interface CustomNodeData {
  label: string
  type: string
  color: string
  file_path?: string
  line_number?: number
  language?: string
  directory?: string
  metadata?: Record<string, string>
}

// Icons for different node types
const TypeIcons: Record<string, string> = {
  file: 'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6z',
  class: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',
  interface: 'M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z',
  function: 'M7 8h10M7 12h10M7 16h10',
  method: 'M5 4h14M5 8h14M5 12h14M5 16h14M5 20h14',
  directory: 'M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z'
}

export const CustomNode = memo(({ data, selected }: NodeProps<CustomNodeData>) => {
  const iconPath = TypeIcons[data.type] || TypeIcons.file

  return (
    <div
      className={`
        px-3 py-2 rounded-lg border-2 shadow-lg min-w-[120px]
        ${selected ? 'border-white' : 'border-transparent'}
        transition-all duration-200 hover:scale-105
      `}
      style={{ backgroundColor: data.color + '22', borderColor: selected ? 'white' : data.color }}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!w-2 !h-2 !bg-gray-400 !border-none"
      />

      <div className="flex items-center gap-2">
        <svg
          className="w-4 h-4"
          fill="none"
          stroke={data.color}
          viewBox="0 0 24 24"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d={iconPath} />
        </svg>
        <span
          className="font-medium text-sm truncate max-w-[150px]"
          style={{ color: data.color }}
          title={data.label}
        >
          {data.label}
        </span>
      </div>

      {/* Type label */}
      <div className="text-[10px] text-gray-400 mt-0.5 flex items-center gap-2">
        <span className="capitalize">{data.type}</span>
        {data.language && (
          <span className="text-gray-500">({data.language})</span>
        )}
        {data.line_number && (
          <span className="text-gray-500">L{data.line_number}</span>
        )}
      </div>

      {/* Directory path for files */}
      {data.directory && data.directory !== '.' && (
        <div className="text-[9px] text-gray-500 truncate max-w-[180px]" title={data.directory}>
          {data.directory}
        </div>
      )}

      <Handle
        type="source"
        position={Position.Bottom}
        className="!w-2 !h-2 !bg-gray-400 !border-none"
      />
    </div>
  )
})

CustomNode.displayName = 'CustomNode'
