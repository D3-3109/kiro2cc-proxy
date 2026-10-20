// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 使用文档页导航（自 codex2cc docsNavigation 适配）：hash 解析与条目搜索

export type DocEntry = {
  id: string
  section: string
  title: string
  summary: string
  method?: string
}

/** hash → 分区/条目目标；未命中时回退 quick-start */
export function resolveDocsTarget(hash: string, entries: DocEntry[]) {
  let id = hash.replace(/^#/, '')
  try {
    id = decodeURIComponent(id)
  } catch {
    id = ''
  }
  const entry = entries.find((entry) => entry.id === id)
  return {
    id: entry?.id || 'quick-start',
    section: entry?.section || 'quick-start',
  }
}

/** 多词 AND 过滤（标题/摘要/方法），最多返回 20 条 */
export function searchDocEntries(query: string, entries: DocEntry[]) {
  const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean)
  if (!words.length) return []
  return entries
    .filter((entry) =>
      words.every((word) =>
        `${entry.title} ${entry.summary} ${entry.method || ''}`
          .toLocaleLowerCase()
          .includes(word),
      ),
    )
    .slice(0, 20)
}
