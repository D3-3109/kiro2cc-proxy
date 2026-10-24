// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 可调用模型白名单多选：选项来自 /api/admin/models，回车可添加列表外的模型 ID
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Check, ChevronDown, Plus, Search, X } from 'lucide-react'
import { useModels } from '@/hooks/use-credentials'

interface ModelMultiSelectProps {
  selected: string[]
  onChange: (models: string[]) => void
}

export function ModelMultiSelect({ selected, onChange }: ModelMultiSelectProps) {
  const { t } = useTranslation()
  const { data } = useModels()
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const rootRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const closeOnOutsideClick = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', closeOnOutsideClick)
    return () => document.removeEventListener('mousedown', closeOnOutsideClick)
  }, [open])

  const knownIds = (data?.data ?? []).map((m) => m.id)
  // 已选但不在上游列表中的手填 ID 也要可见、可取消
  const optionIds = [...knownIds, ...selected.filter((id) => !knownIds.includes(id))]
  const trimmedQuery = query.trim()
  const filtered = optionIds.filter((id) => id.toLowerCase().includes(trimmedQuery.toLowerCase()))
  const canAddCustom = trimmedQuery !== '' && !optionIds.includes(trimmedQuery)

  const toggle = (id: string) => {
    onChange(selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id])
  }

  const addCustom = () => {
    if (!canAddCustom) return
    onChange([...selected, trimmedQuery])
    setQuery('')
  }

  return (
    <div className="relative mt-2" ref={rootRef}>
      <button
        type="button"
        onClick={() => { setOpen(!open); setQuery('') }}
        aria-expanded={open}
        className="w-full flex items-center justify-between gap-2 rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm hover:bg-accent/50 transition-colors"
      >
        <div className="flex flex-wrap gap-1 flex-1 min-w-0">
          {selected.length === 0 ? (
            <span className="text-muted-foreground">{t('apiKeys.allModelsAllowed')}</span>
          ) : (
            selected.map((id) => (
              <span
                key={id}
                className="inline-flex items-center gap-1 rounded-full bg-violet-100 dark:bg-violet-900/50 text-violet-700 dark:text-violet-300 border border-violet-200 dark:border-violet-700 px-2 py-0.5 text-xs font-medium"
              >
                {id}
                <span
                  role="button"
                  tabIndex={0}
                  aria-label={t('apiKeys.removeModel', { model: id })}
                  className="hover:text-destructive cursor-pointer"
                  onClick={(e) => { e.stopPropagation(); toggle(id) }}
                  onKeyDown={(e) => e.key === 'Enter' && toggle(id)}
                >
                  <X className="h-3 w-3" />
                </span>
              </span>
            ))
          )}
        </div>
        <ChevronDown className={`h-4 w-4 shrink-0 text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div className="absolute z-50 mt-1 w-full rounded-md border bg-popover shadow-md">
          <div className="p-2 border-b">
            <div className="relative">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <input
                autoFocus
                type="text"
                aria-label={t('apiKeys.searchModelPlaceholder')}
                placeholder={t('apiKeys.searchModelPlaceholder')}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key !== 'Enter') return
                  e.preventDefault()
                  addCustom()
                }}
                className="w-full rounded-sm border-0 bg-transparent pl-7 pr-2 py-1 text-sm outline-none placeholder:text-muted-foreground"
              />
            </div>
          </div>
          <div className="max-h-48 overflow-y-auto py-1">
            {canAddCustom && (
              <button
                type="button"
                onClick={addCustom}
                className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-accent transition-colors text-left"
              >
                <Plus className="h-4 w-4 shrink-0" />
                {t('apiKeys.addCustomModel', { model: trimmedQuery })}
              </button>
            )}
            {filtered.length === 0 && !canAddCustom ? (
              <div className="px-3 py-2 text-sm text-muted-foreground">{t('apiKeys.noMatchingModels')}</div>
            ) : (
              filtered.map((id) => {
                const isSelected = selected.includes(id)
                return (
                  <button
                    key={id}
                    type="button"
                    onClick={() => toggle(id)}
                    aria-pressed={isSelected}
                    className={`w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-accent transition-colors text-left ${isSelected ? 'bg-violet-50 dark:bg-violet-950/30' : ''}`}
                  >
                    <div className={`h-4 w-4 shrink-0 rounded border flex items-center justify-center ${isSelected ? 'bg-violet-600 border-violet-600 text-white' : 'border-input'}`}>
                      {isSelected && <Check className="h-3 w-3" />}
                    </div>
                    <span className="font-mono text-xs">{id}</span>
                  </button>
                )
              })
            )}
          </div>
        </div>
      )}
    </div>
  )
}
