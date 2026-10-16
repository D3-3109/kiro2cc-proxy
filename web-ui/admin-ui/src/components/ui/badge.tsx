import * as React from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const badgeVariants = cva(
  // 设计稿 .tag：20px 高 / 6px 圆角 / 11px 字号 / 5px 间隙（pip 与文字）
  'inline-flex h-5 items-center gap-[5px] whitespace-nowrap rounded-[6px] border border-transparent px-[7px] text-[11px] font-semibold transition-colors',
  {
    variants: {
      variant: {
        /** .tag.new */
        default: 'bg-brand-soft text-brand',
        /** .tag.off */
        secondary: 'bg-surface-3 text-ink-3',
        /** .tag.rate */
        destructive: 'bg-danger-soft text-danger',
        outline: 'text-ink-2',
        /** .tag.ok */
        success: 'bg-ok-soft text-ok',
        /** .tag.warn */
        warning: 'bg-warn-soft text-warn',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
)

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  )
}

export { Badge, badgeVariants }
