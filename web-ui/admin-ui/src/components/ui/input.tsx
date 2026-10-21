import * as React from 'react'
import { cn } from '@/lib/utils'

/** 输入框外观（设计稿 .field）：31px 高 / 7px 圆角 / surface-2 底 / hairline-2 边
 *  聚焦态与禁用降权由两个分支各自补齐 —— 必须落在承载边框/背景的那一层上 */
export const FIELD_BOX =
  'h-[31px] rounded-[7px] border-[0.5px] border-hairline-2 bg-surface-2 px-2.5 text-[12px] text-ink transition-colors'
/** 输入控件本体：去边框去背景，交由 FIELD_BOX 承载视觉 */
const FIELD_CONTROL =
  'bg-transparent outline-none placeholder:text-ink-3 disabled:cursor-not-allowed'

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'prefix'> {
  /** 右侧单位标签（设计稿 .field .u）；传入时输入框包一层外壳，className 落在内层 input */
  unit?: React.ReactNode
  /** 左侧前缀（通常为图标）；与 unit 共用同一外壳分支，不传时与无前缀行为一致。
   *  原生 <input prefix> 是字符串属性（待废弃的输入提示），此处覆写为节点类型并从透传中排除 */
  prefix?: React.ReactNode
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, unit, prefix, ...props }, ref) => {
    if (unit || prefix) {
      return (
        <div
          className={cn(
            FIELD_BOX,
            // 焦点与禁用态由外壳承载：input 聚焦时命中 :focus-within，input 禁用时命中 :has(:disabled)
            'flex w-full items-center gap-2 focus-within:border-brand has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-50'
          )}
        >
          {prefix && <span className="shrink-0 text-ink-3">{prefix}</span>}
          <input
            type={type}
            className={cn(FIELD_CONTROL, 'min-w-0 flex-1', className)}
            ref={ref}
            {...props}
          />
          {unit && <span className="shrink-0 text-[11px] text-ink-3">{unit}</span>}
        </div>
      )
    }
    return (
      <input
        type={type}
        className={cn(
          FIELD_BOX,
          FIELD_CONTROL,
          // 无外壳时 input 自身即边框层，用 :focus 而非 :focus-within 表意更准确
          'w-full focus:border-brand disabled:opacity-50',
          className
        )}
        ref={ref}
        {...props}
      />
    )
  }
)
Input.displayName = 'Input'

export { Input }
