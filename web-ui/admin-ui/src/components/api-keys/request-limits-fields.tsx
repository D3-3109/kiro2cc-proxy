// Copyright (c) 2026 Harllan He. Licensed under MIT.
// API Key 的 RPM / 并发上限输入（创建与编辑对话框共用），0 = 不限
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'

export interface RequestLimits {
  rpmLimit: number
  concurrencyLimit: number
}

export const UNLIMITED_REQUEST_LIMITS: RequestLimits = { rpmLimit: 0, concurrencyLimit: 0 }

interface RequestLimitsFieldsProps {
  value: RequestLimits
  onChange: (limits: RequestLimits) => void
}

export function RequestLimitsFields({ value, onChange }: RequestLimitsFieldsProps) {
  const { t } = useTranslation()

  const field = (key: keyof RequestLimits, label: string) => (
    <label className="flex items-center gap-2">
      <span className="text-sm text-muted-foreground">{label}</span>
      <Input
        type="text"
        inputMode="numeric"
        value={value[key] || ''}
        placeholder="0"
        onChange={(e) => {
          const digits = e.target.value.replace(/\D/g, '')
          onChange({ ...value, [key]: digits === '' ? 0 : Number(digits) })
        }}
        onFocus={(e) => e.target.select()}
        className="w-24"
      />
    </label>
  )

  return (
    <div>
      <span className="text-sm font-medium">{t('apiKeys.requestLimitsLabel')}</span>
      <p className="text-xs text-muted-foreground mt-0.5">{t('apiKeys.requestLimitsHint')}</p>
      <div className="flex flex-wrap gap-4 mt-2">
        {field('rpmLimit', t('apiKeys.rpmLimit'))}
        {field('concurrencyLimit', t('apiKeys.concurrencyLimit'))}
      </div>
    </div>
  )
}
