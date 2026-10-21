// Copyright (c) 2026 Harllan He. Licensed under MIT.
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

/**
 * 凭据表单（添加 / 编辑账号弹窗）共享视觉基元 —— 单点定义，两处只 import 不复制。
 *
 * 分区卡片说明：settings-panel.tsx 的 .set-card 用 border-transparent bg-surface shadow-hair，
 * 依赖页面底色 --background 与卡片 --surface 的色差 + 投影区分层级。弹窗背景本身就是
 * bg-surface，照搬会让卡片边界完全消失，故这里保留其圆角/内边距语义，把层级改由
 * hairline 描边提供（见 SECTION_CARD）。
 */

/** 分区标题（对齐设计稿 .set-cap） */
export const SECTION_CAP =
  'flex items-center gap-1.5 px-0.5 pb-2 text-[10.5px] font-semibold uppercase tracking-[.08em] text-ink-3'

/** 分区卡片：弹窗内是 bg-surface 之上再叠 bg-surface，边界只由 hairline 描边提供 */
export const SECTION_CARD = 'rounded-lg border-[0.5px] border-hairline bg-surface p-4'

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

/** 分组：分区标题 + 描边卡片 */
export function Section({ title, children }: { title: ReactNode; children: ReactNode }) {
  return (
    <section>
      <div className={SECTION_CAP}>{title}</div>
      <div className={SECTION_CARD}>
        <div className={FIELD_GROUP}>{children}</div>
      </div>
    </section>
  )
}
