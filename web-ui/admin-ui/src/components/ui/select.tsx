import * as React from 'react'
import { ChevronDown } from 'lucide-react'
import { cn } from '@/lib/utils'
import { FIELD_BOX } from '@/components/ui/input'

/**
 * 下拉字段（设计稿 .field 的下拉形态）。
 *
 * 与 Input 共用 FIELD_BOX —— 外壳即边框层，原生 <select> 只承载文字与交互：
 * - appearance-none 关掉浏览器 UA 箭头，改由右侧绝对定位的 ChevronDown 提供，
 *   箭头随主题 token 取色；原生 <select> 元素与 id/name 全部保留，键盘与读屏行为不变。
 * - select 自身不设 h-*，撑满外壳 31px；pr-8 预留箭头槽位，避免长选项文字压到图标。
 */
type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>

const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, children, ...props }, ref) => (
    <div
      className={cn(
        FIELD_BOX,
        'relative flex items-center focus-within:border-brand has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-50'
      )}
    >
      <select
        ref={ref}
        className={cn(
          'h-full w-full appearance-none bg-transparent pr-8 text-ink outline-none disabled:cursor-not-allowed',
          className
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronDown
        aria-hidden="true"
        className="pointer-events-none absolute right-2.5 size-[13px] text-ink-3"
      />
    </div>
  )
)
Select.displayName = 'Select'

export { Select }
