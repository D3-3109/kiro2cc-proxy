// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 使用文档页右侧本页目录（父子两级 + aria-current）

export type DocsTOCItem = {
  id: string
  label: string
  children?: { id: string; label: string; method?: string }[]
}

export default function DocsTOC({
  items,
  title,
  activeId,
}: {
  items: DocsTOCItem[]
  title: string
  activeId: string
}) {
  return (
    <nav className="docs-toc" aria-label={title}>
      <div className="mb-2 px-2 text-[10.5px] font-semibold uppercase tracking-[.08em] text-ink-3">
        {title}
      </div>
      {items.map((item) => (
        <div key={item.id} className="mb-0.5">
          <a
            href={`#${item.id}`}
            aria-current={activeId === item.id ? 'location' : undefined}
            className={`block rounded-[6px] px-2 py-[5px] text-[12px] font-medium transition-colors ${
              activeId === item.id
                ? 'bg-brand-soft font-semibold text-brand'
                : 'text-ink-2 hover:bg-surface-3 hover:text-ink'
            }`}
          >
            {item.label}
          </a>
          {item.children?.map((child) => (
            <a
              key={child.id}
              href={`#${child.id}`}
              aria-current={activeId === child.id ? 'location' : undefined}
              className={`flex items-center gap-1.5 rounded-[6px] px-2 py-[4px] pl-[26px] text-[11.5px] transition-colors ${
                activeId === child.id
                  ? 'bg-brand-soft font-semibold text-brand'
                  : 'text-ink-3 hover:bg-surface-3 hover:text-ink'
              }`}
            >
              {child.method && (
                <span className="font-mono text-[10px] font-bold text-ink-3">{child.method}</span>
              )}
              <span className="truncate">{child.label}</span>
            </a>
          ))}
        </div>
      ))}
    </nav>
  )
}
