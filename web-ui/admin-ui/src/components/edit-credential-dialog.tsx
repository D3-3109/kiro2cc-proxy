// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState, useEffect } from 'react'
import { toast } from 'sonner'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/select'
import { Field, Section } from '@/components/credential-form-kit'
import { useSetPriority, useUpdateCredential } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import { normalizeLoginSource } from '@/lib/account-state'
import type { CredentialStatusItem } from '@/types/api'

interface EditCredentialDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  credential: CredentialStatusItem
}

export function EditCredentialDialog({ open, onOpenChange, credential }: EditCredentialDialogProps) {
  const { t } = useTranslation()
  const [authRegion, setAuthRegion] = useState('')
  const [apiRegion, setApiRegion] = useState('')
  const [nickname, setNickname] = useState('')
  const [email, setEmail] = useState('')
  const [clientId, setClientId] = useState('')
  const [clientSecret, setClientSecret] = useState('')
  const [profileArn, setProfileArn] = useState('')
  const [machineId, setMachineId] = useState('')
  const [proxyUrl, setProxyUrl] = useState('')
  const [proxyUsername, setProxyUsername] = useState('')
  const [proxyPassword, setProxyPassword] = useState('')
  const [priority, setPriority] = useState('')
  // 空串代表「不指定」，与 normalizeLoginSource 的 undefined 语义对齐
  const [loginSourceValue, setLoginSourceValue] = useState('')

  const { mutateAsync: updateAsync, isPending: updatePending } = useUpdateCredential()
  const { mutateAsync: setPriorityAsync, isPending: priorityPending } = useSetPriority()
  // 两个端点串行提交，任一进行中即锁表单
  const isPending = updatePending || priorityPending

  // 仅在对话框打开时按当前凭据重置表单。
  // 依赖取 credential.id 而非整个 credential 对象 —— 列表每 30s 自动刷新，
  // TanStack Query 会为数据有变动的行生成新引用，依赖整个对象会在用户编辑中途静默清空已输入内容。
  useEffect(() => {
    if (open) {
      setAuthRegion('')
      setApiRegion('')
      setNickname(credential.nickname || '')
      setEmail(credential.email || '')
      setClientId('')
      setClientSecret('')
      setProfileArn('')
      setMachineId('')
      setProxyUrl(credential.proxyUrl || '')
      setProxyUsername('')
      setProxyPassword('')
      setPriority(String(credential.priority))
      setLoginSourceValue(normalizeLoginSource(credential.provider) ?? '')
    }
  }, [open, credential.id])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    // 优先级不在 UpdateCredentialRequest 里，走独立端点；先校验避免半途失败
    const priorityChanged = priority !== String(credential.priority)
    if (priorityChanged && !/^\d+$/.test(priority)) {
      toast.error(t('credentials.toastPriorityInvalid'))
      return
    }

    // 构建只包含有变更的字段
    const data: Record<string, string> = {}
    if (authRegion !== '') data.authRegion = authRegion
    if (apiRegion !== '') data.apiRegion = apiRegion
    if (nickname !== (credential.nickname || '')) data.nickname = nickname
    if (email !== (credential.email || '')) data.email = email
    if (clientId !== '') data.clientId = clientId
    if (clientSecret !== '') data.clientSecret = clientSecret
    if (profileArn !== '') data.profileArn = profileArn
    if (machineId !== '') data.machineId = machineId
    if (proxyUrl !== (credential.proxyUrl || '')) data.proxyUrl = proxyUrl
    // 代理地址被清空时同步清除认证信息：后端对 username/password 是「留空不修改」语义，
    // 只清地址会持久化出「无代理但残留凭据」的脏数据
    if (proxyUrl === '' && (credential.proxyUrl || '') !== '') {
      data.proxyUsername = ''
      data.proxyPassword = ''
    } else {
      if (proxyUsername !== '') data.proxyUsername = proxyUsername
      if (proxyPassword !== '') data.proxyPassword = proxyPassword
    }
    // 登录来源：仅 social 账号可改；空串表示清除。不写 authMethod —— 后端把它也算作身份变更，会白清 subscription_title
    if (isSocial && loginSourceValue !== currentProvider) data.provider = loginSourceValue

    if (!priorityChanged && Object.keys(data).length === 0) {
      toast.info(t('credentials.toastNoFieldsToUpdate'))
      return
    }

    const done: string[] = []
    try {
      if (priorityChanged) {
        done.push((await setPriorityAsync({ id: credential.id, priority: Number(priority) })).message)
      }
      if (Object.keys(data).length > 0) {
        done.push((await updateAsync({ id: credential.id, data })).message)
      }
      toast.success(done.join(' · '))
      onOpenChange(false)
    } catch (error) {
      // 串行提交下优先级先落地：失败时把已成功的部分一并告知，避免用户重复设置
      const failed = t('credentials.toastUpdateFailed', { message: extractErrorMessage(error) })
      toast.error(done.length > 0 ? `${done.join(' · ')} · ${failed}` : failed)
    }
  }

  const isIdc = credential.authMethod === 'idc'
  // 仅 social 账号的 provider 是「登录来源」；external_idp 的 provider 存的是 IdP 名称，不可被此处覆盖
  const isSocial = credential.authMethod === 'social'
  // 存量账号 provider 可能为空或大小写不固定，统一归一化后再比较，避免无变更时误提交
  const currentProvider = normalizeLoginSource(credential.provider) ?? ''

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t('credentials.editDialogTitle', { id: credential.id })}</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="flex flex-col min-h-0 flex-1">
          <div className="space-y-5 overflow-y-auto flex-1 pr-1 py-1">
            <p className="text-[11px] leading-[1.55] text-ink-3">
              {t('credentials.editHintOnlyChanged')}
            </p>

            {/* 核心凭据 */}
            <Section title={t('credentials.sectionCore')}>
              <Field label={t('credentials.nicknameLabel')} htmlFor="nickname">
                <Input
                  id="nickname"
                  placeholder={t('credentials.nicknamePlaceholder')}
                  value={nickname}
                  onChange={(e) => setNickname(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field label={t('credentials.emailLabel')} htmlFor="email">
                <Input
                  id="email"
                  placeholder={t('credentials.emailPlaceholderEdit')}
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              {/* 登录来源（仅 social；IdC 的 BuilderId 由 authMethod 推断、external_idp 的 provider 为 IdP 名称，二者均不可改） */}
              {isSocial && (
                <Field
                  label={t('credentials.loginSourceLabel')}
                  htmlFor="loginSource"
                  hint={t('credentials.loginSourceHintEdit')}
                >
                  <Select
                    id="loginSource"
                    value={loginSourceValue}
                    onChange={(e) => setLoginSourceValue(e.target.value)}
                    disabled={isPending}
                  >
                    <option value="">{t('credentials.loginSourceUnspecified')}</option>
                    <option value="Google">Google</option>
                    <option value="GitHub">GitHub</option>
                  </Select>
                </Field>
              )}
            </Section>

            {/* 身份与区域 */}
            <Section title={t('credentials.sectionIdentityRegion')}>
              <Field label={t('credentials.regionConfigLabel')} hint={t('credentials.regionHintEdit')}>
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    id="authRegion"
                    placeholder={t('credentials.authRegionPlaceholder')}
                    value={authRegion}
                    onChange={(e) => setAuthRegion(e.target.value)}
                    disabled={isPending}
                  />
                  <Input
                    id="apiRegion"
                    placeholder={t('credentials.apiRegionPlaceholder')}
                    value={apiRegion}
                    onChange={(e) => setApiRegion(e.target.value)}
                    disabled={isPending}
                  />
                </div>
              </Field>

              {/* IdC 字段 */}
              {isIdc && (
                <>
                  <Field label={t('credentials.clientIdLabel')} htmlFor="clientId">
                    <Input
                      id="clientId"
                      placeholder={t('credentials.leaveBlankNoChange')}
                      value={clientId}
                      onChange={(e) => setClientId(e.target.value)}
                      disabled={isPending}
                    />
                  </Field>
                  <Field label={t('credentials.clientSecretLabel')} htmlFor="clientSecret">
                    <Input
                      id="clientSecret"
                      type="password"
                      placeholder={t('credentials.leaveBlankNoChange')}
                      value={clientSecret}
                      onChange={(e) => setClientSecret(e.target.value)}
                      disabled={isPending}
                    />
                  </Field>
                </>
              )}

              <Field
                label={t('credentials.profileArnLabel')}
                htmlFor="profileArn"
                hint={t('credentials.profileArnHintEdit')}
              >
                <Input
                  id="profileArn"
                  placeholder={credential.hasProfileArn ? t('credentials.profileArnConfiguredPlaceholder') : t('credentials.leaveBlankNoChange')}
                  value={profileArn}
                  onChange={(e) => setProfileArn(e.target.value)}
                  disabled={isPending}
                />
              </Field>
            </Section>

            {/* 运行与网络 */}
            <Section title={t('credentials.sectionRuntimeNetwork')}>
              <Field
                label={t('credentials.priorityFieldLabel')}
                htmlFor="priority"
                hint={t('credentials.priorityHint')}
              >
                <Input
                  id="priority"
                  type="number"
                  min={0}
                  step={1}
                  inputMode="numeric"
                  placeholder={t('credentials.priorityPlaceholder')}
                  value={priority}
                  onChange={(e) => setPriority(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field label={t('credentials.machineIdLabel')} htmlFor="machineId">
                <Input
                  id="machineId"
                  placeholder={t('credentials.leaveBlankNoChange')}
                  value={machineId}
                  onChange={(e) => setMachineId(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field label={t('credentials.proxyConfigLabel')} htmlFor="proxyUrl">
                <Input
                  id="proxyUrl"
                  placeholder={t('credentials.proxyUrlPlaceholderEdit')}
                  value={proxyUrl}
                  onChange={(e) => setProxyUrl(e.target.value)}
                  disabled={isPending}
                />
                <div className="grid grid-cols-2 gap-2 mt-2">
                  <Input
                    id="proxyUsername"
                    placeholder={t('credentials.proxyUsernamePlaceholder')}
                    value={proxyUsername}
                    onChange={(e) => setProxyUsername(e.target.value)}
                    disabled={isPending}
                  />
                  <Input
                    id="proxyPassword"
                    type="password"
                    placeholder={t('credentials.proxyPasswordPlaceholder')}
                    value={proxyPassword}
                    onChange={(e) => setProxyPassword(e.target.value)}
                    disabled={isPending}
                  />
                </div>
              </Field>
            </Section>
          </div>

          <DialogFooter className="pt-4">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isPending}
            >
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={isPending}>
              {isPending ? t('credentials.updatingButton') : t('common.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
