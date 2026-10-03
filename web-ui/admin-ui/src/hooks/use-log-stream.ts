// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState, useEffect, useCallback, useRef } from 'react'
import { storage } from '@/lib/storage'

/** log 事件的合批窗口（ms）：窗口内的日志只触发一次 setState / re-render */
const FLUSH_INTERVAL_MS = 150

export interface LogEntry {
  timestamp: string
  level: 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'
  target: string
  message: string
}

/** 前端环形缓冲上限；页面侧的容量指标卡直接复用此常量 */
export const MAX_FRONT_LOGS = 2000

export function useLogStream(enabled: boolean): {
  logs: LogEntry[]
  connected: boolean
  /** 清空缓冲：必须由 hook 内部清，否则下一条日志到达时旧内容会整体回填 */
  clear: () => void
} {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [connected, setConnected] = useState(false)
  const esRef = useRef<EventSource | null>(null)
  const mountedRef = useRef(true)
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const reconnectDelay = useRef(1000)

  const historyReceivedRef = useRef(false)
  const pendingRef = useRef<LogEntry[]>([])
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  /** 把合批缓冲一次性并入 logs；clear/断连时也要调用以避免旧内容回填 */
  const flush = useCallback(() => {
    flushTimerRef.current = null
    if (pendingRef.current.length === 0) return
    const incoming = pendingRef.current
    pendingRef.current = []
    setLogs((prev) => {
      const next = prev.length + incoming.length > MAX_FRONT_LOGS
        ? [...prev, ...incoming].slice(-(MAX_FRONT_LOGS as number))
        : [...prev, ...incoming]
      return next
    })
  }, [])

  const clear = useCallback(() => {
    pendingRef.current = []
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current)
      flushTimerRef.current = null
    }
    setLogs([])
  }, [])

  const connect = useCallback(() => {
    if (esRef.current) return
    const apiKey = storage.getApiKey()
    if (!apiKey) return

    historyReceivedRef.current = false

    const es = new EventSource(
      `/api/admin/logs/stream?api_key=${encodeURIComponent(apiKey)}`
    )
    esRef.current = es

    es.onopen = () => {
      setConnected(true)
      reconnectDelay.current = 1000
      // REST 获取初始快照，规避反向代理对 SSE body 的缓冲
      fetch(`/api/admin/logs/snapshot?api_key=${encodeURIComponent(apiKey!)}`)
        .then((r) => r.json())
        .then((entries: LogEntry[]) => {
          if (!historyReceivedRef.current && Array.isArray(entries)) {
            historyReceivedRef.current = true
            setLogs(entries)
          }
        })
        .catch(() => {})
    }

    es.addEventListener('history', (e: MessageEvent) => {
      try {
        const entries: LogEntry[] = JSON.parse(e.data)
        if (Array.isArray(entries)) {
          historyReceivedRef.current = true
          setLogs(entries)
        }
      } catch {
        // ignore malformed history payload
      }
    })

    es.addEventListener('log', (e: MessageEvent) => {
      try {
        const entry: LogEntry = JSON.parse(e.data)
        // 合批缓冲：SSE 逐条推送时每条一次 setState + O(n) 复制会让页面在
        // 日志多时整段卡顿；先攒入 pending 再由定时器一次性 flush。
        pendingRef.current.push(entry)
        // 缓冲加上限：上游日志洪峰时避免 pending 无界增长
        if (pendingRef.current.length > MAX_FRONT_LOGS) pendingRef.current.shift()
        if (!flushTimerRef.current) {
          flushTimerRef.current = setTimeout(flush, FLUSH_INTERVAL_MS)
        }
      } catch {
        // ignore malformed log entry
      }
    })

    es.onerror = () => {
      if (!mountedRef.current) return
      setConnected(false)
      es.close()
      esRef.current = null
      const delay = reconnectDelay.current
      reconnectDelay.current = Math.min(delay * 2, 30000)
      reconnectTimer.current = setTimeout(connect, delay)
    }
  }, [flush])

  useEffect(() => {
    if (!enabled) {
      esRef.current?.close()
      esRef.current = null
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current)
      setConnected(false)
      pendingRef.current = []
      if (flushTimerRef.current) {
        clearTimeout(flushTimerRef.current)
        flushTimerRef.current = null
      }
      setLogs([])
      return
    }

    connect()

    return () => {
      mountedRef.current = false
      esRef.current?.close()
      esRef.current = null
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current)
      if (flushTimerRef.current) {
        clearTimeout(flushTimerRef.current)
        flushTimerRef.current = null
      }
      pendingRef.current = []
    }
  }, [enabled, connect])

  return { logs, connected, clear }
}
