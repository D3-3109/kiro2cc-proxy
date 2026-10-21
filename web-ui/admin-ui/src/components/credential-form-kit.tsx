// Copyright (c) 2026 Harllan He. Licensed under MIT.
import type { ReactNode } from 'react'
import type { LucideIcon } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { FIELD_BOX } from '@/components/ui/input'
import { cn } from '@/lib/utils'

/**
 * 凭据表单（添加 / 编辑账号弹窗）共享视觉基元 —— 单点定义，两处只 import 不复制。
 *
 * 分区卡片说明：settings-panel.tsx 的 .set-card 用 border-transparent bg-surface shadow-hair，
 * 依赖页面底色 --background 与卡片 --surface 的色差 + 投影区分层级。弹窗背景本身就是
 * bg-surface，照搬会让卡片边界完全消失，故层级改由 surface-2 底色与 hairline 描边
 * 双通道提供（见 SECTION_CARD）。
 */

/** 分区标题（对齐设计稿 .set-cap） */
export const SECTION_CAP =
  'flex items-center gap-1.5 px-0.5 pb-2 text-[10.5px] font-semibold uppercase tracking-[.08em] text-ink-3'

/** 分区卡片：surface-2 底色与弹窗 bg-surface 拉开层次，hairline 描边兜底 */
export const SECTION_CARD = 'rounded-lg border-[0.5px] border-hairline bg-surface-2 p-4'

/** 字段标签 */
export const FIELD_LABEL = 'block text-[11.5px] font-medium text-ink-2'

/** 标签与输入框的间距（不用容器 gap，避免与 hint 间距叠加） */
export const FIELD_LABEL_SPACING = 'mb-2'

/** 字段说明文字 */
export const FIELD_HINT = 'text-[11px] leading-[1.55] text-ink-3'

/** 说明文字与所属输入框的间距：6px，须明显小于字段之间的 space-y-4（16px） */
export const FIELD_HINT_SPACING = 'mt-1.5'

/** 组内字段纵向间距 */
export const FIELD_GROUP = 'space-y-4'

/** 只读字段壳：直接复用 Input 的 FIELD_BOX（尺寸/底色/描边同源，不会漂移），
 *  只叠加只读所需的排版差异 —— 文字降为 ink-2，truncate 单行省略 */
const READONLY_BOX = cn(FIELD_BOX, 'flex items-center gap-1.5 truncate text-ink-2')

/** 只读字段：账号已有值的锁定态。value 为空串时不该被用到——调用方须以有值为渲染前提 */
export function ReadOnlyField({
  label,
  value,
  lockNote,
  mono,
  title,
}: {
  /** 字段标签；与可编辑 Input 并排、由外层 Field 统一承载标签时省略，避免空容器撑出高度使两侧错位 */
  label?: ReactNode
  value: string
  /** 锁定说明徽标（如「已锁定」），贴右展示 */
  lockNote?: string
  /** 值域为标识符/ARN 时用等宽字体 */
  mono?: boolean
  /** 悬停完整值（默认 value 本身） */
  title?: string
}) {
  return (
    <div>
      {label && <div className={`${FIELD_LABEL} ${FIELD_LABEL_SPACING}`}>{label}</div>}
      <div className={mono ? `${READONLY_BOX} font-mono` : READONLY_BOX} title={title ?? value}>
        <span className="truncate">{value}</span>
        {lockNote && (
          <span className="ml-auto flex-none text-[10px] text-ink-3">{lockNote}</span>
        )}
      </div>
    </div>
  )
}

/** 单个字段块：标签 + 控件 + 可选说明。不用容器 gap，间距由 label/hint 各自 margin 提供 */
export function Field({
  label,
  htmlFor,
  required,
  hint,
  children,
}: {
  label: ReactNode
  htmlFor?: string
  required?: boolean
  hint?: ReactNode
  children: ReactNode
}) {
  const { t } = useTranslation()
  return (
    <div>
      <label htmlFor={htmlFor} className={`${FIELD_LABEL} ${FIELD_LABEL_SPACING}`}>
        {label}
        {/* 星号纯视觉：读屏会把它念进标签名却播报不出「必填」，故对 AT 隐藏，
            改由 sr-only 文案提供必填语义（控件侧的校验仍走既有 toast，不注入原生 required） */}
        {required && (
          <>
            <span className="text-danger" aria-hidden="true"> *</span>
            <span className="sr-only">{t('credentials.required')}</span>
          </>
        )}
      </label>
      {children}
      {hint && <p className={`${FIELD_HINT} ${FIELD_HINT_SPACING}`}>{hint}</p>}
    </div>
  )
}

/** 分组：分区标题（可选语义图标）+ 卡片 */
export function Section({
  icon: Icon,
  title,
  children,
}: {
  icon?: LucideIcon
  title: ReactNode
  children: ReactNode
}) {
  return (
    <section>
      <div className={SECTION_CAP}>
        {Icon && <Icon className="size-3" aria-hidden="true" />}
        {title}
      </div>
      <div className={SECTION_CARD}>
        <div className={FIELD_GROUP}>{children}</div>
      </div>
    </section>
  )
}
