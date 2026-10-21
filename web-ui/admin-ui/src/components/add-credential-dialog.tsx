// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState } from 'react'
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
import { useAddCredential } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import { normalizeLoginSource } from '@/lib/account-state'

interface AddCredentialDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type AuthMethod = 'social' | 'idc'

export function AddCredentialDialog({ open, onOpenChange }: AddCredentialDialogProps) {
  const { t } = useTranslation()
  const [refreshToken, setRefreshToken] = useState('')
  const [email, setEmail] = useState('')
  const [authMethod, setAuthMethod] = useState<AuthMethod>('social')
  const [loginSource, setLoginSource] = useState('')
  const [authRegion, setAuthRegion] = useState('')
  const [apiRegion, setApiRegion] = useState('')
  const [clientId, setClientId] = useState('')
  const [clientSecret, setClientSecret] = useState('')
  const [profileArn, setProfileArn] = useState('')
  const [priority, setPriority] = useState('0')
  const [machineId, setMachineId] = useState('')
  const [proxyUrl, setProxyUrl] = useState('')
  const [proxyUsername, setProxyUsername] = useState('')
  const [proxyPassword, setProxyPassword] = useState('')

  const { mutate, isPending } = useAddCredential()

  const resetForm = () => {
    setRefreshToken('')
    setEmail('')
    setAuthMethod('social')
    setLoginSource('')
    setAuthRegion('')
    setApiRegion('')
    setClientId('')
    setClientSecret('')
    setProfileArn('')
    setPriority('0')
    setMachineId('')
    setProxyUrl('')
    setProxyUsername('')
    setProxyPassword('')
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    // 验证必填字段
    if (!refreshToken.trim()) {
      toast.error(t('credentials.toastRefreshTokenRequired'))
      return
    }

    // IdC/Builder-ID/IAM 需要额外字段
    if (authMethod === 'idc' && (!clientId.trim() || !clientSecret.trim())) {
      toast.error(t('credentials.toastIdcFieldsRequired'))
      return
    }

    mutate(
      {
        refreshToken: refreshToken.trim(),
        authMethod,
        // 仅在 social 下提交：切到 idc 后 loginSource 不会自动清空，无条件下发会把 IdC 账号写成脏数据
        provider: authMethod === 'social' ? normalizeLoginSource(loginSource) : undefined,
        email: email.trim() || undefined,
        authRegion: authRegion.trim() || undefined,
        apiRegion: apiRegion.trim() || undefined,
        clientId: clientId.trim() || undefined,
        clientSecret: clientSecret.trim() || undefined,
        profileArn: profileArn.trim() || undefined,
        priority: parseInt(priority) || 0,
        machineId: machineId.trim() || undefined,
        proxyUrl: proxyUrl.trim() || undefined,
        proxyUsername: proxyUsername.trim() || undefined,
        proxyPassword: proxyPassword.trim() || undefined,
      },
      {
        onSuccess: (data) => {
          toast.success(data.message)
          onOpenChange(false)
          resetForm()
        },
        onError: (error: unknown) => {
          toast.error(t('credentials.toastAddFailed', { message: extractErrorMessage(error) }))
        },
      }
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t('credentials.addDialogTitle')}</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="flex flex-col min-h-0 flex-1">
          <div className="space-y-5 overflow-y-auto flex-1 pr-1 py-1">
            {/* 核心凭据 */}
            <Section title={t('credentials.sectionCore')}>
              <Field
                label={t('credentials.refreshTokenLabel')}
                htmlFor="refreshToken"
                required
              >
                <Input
                  id="refreshToken"
                  type="password"
                  placeholder={t('credentials.refreshTokenPlaceholder')}
                  value={refreshToken}
                  onChange={(e) => setRefreshToken(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field label={t('credentials.emailLabel')} htmlFor="email">
                <Input
                  id="email"
                  type="text"
                  placeholder={t('credentials.emailPlaceholderAdd')}
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field label={t('credentials.authMethodLabel')} htmlFor="authMethod">
                <Select
                  id="authMethod"
                  value={authMethod}
                  onChange={(e) => setAuthMethod(e.target.value as AuthMethod)}
                  disabled={isPending}
                >
                  <option value="social">Social</option>
                  <option value="idc">IdC/Builder-ID/IAM</option>
                </Select>
              </Field>

              {/* 登录来源（仅 social 需要；IdC 的 BuilderId 由 authMethod 推断） */}
              {authMethod === 'social' && (
                <Field label={t('credentials.loginSourceLabel')} htmlFor="loginSource">
                  <Select
                    id="loginSource"
                    value={loginSource}
                    onChange={(e) => setLoginSource(e.target.value)}
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
              <Field label={t('credentials.regionConfigLabel')} hint={t('credentials.regionHintAdd')}>
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

              {/* IdC/Builder-ID/IAM 额外字段 */}
              {authMethod === 'idc' && (
                <>
                  <Field
                    label={t('credentials.clientIdLabel')}
                    htmlFor="clientId"
                    required
                  >
                    <Input
                      id="clientId"
                      placeholder={t('credentials.clientIdPlaceholderAdd')}
                      value={clientId}
                      onChange={(e) => setClientId(e.target.value)}
                      disabled={isPending}
                    />
                  </Field>
                  <Field
                    label={t('credentials.clientSecretLabel')}
                    htmlFor="clientSecret"
                    required
                  >
                    <Input
                      id="clientSecret"
                      type="password"
                      placeholder={t('credentials.clientSecretPlaceholderAdd')}
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
                hint={t('credentials.profileArnHintAdd')}
              >
                <Input
                  id="profileArn"
                  placeholder="arn:aws:codewhisperer:<region>:<account-id>:profile/<profile-id>"
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
                  min="0"
                  placeholder={t('credentials.priorityPlaceholder')}
                  value={priority}
                  onChange={(e) => setPriority(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field
                label={t('credentials.machineIdLabel')}
                htmlFor="machineId"
                hint={t('credentials.machineIdHint')}
              >
                <Input
                  id="machineId"
                  placeholder={t('credentials.machineIdPlaceholderAdd')}
                  value={machineId}
                  onChange={(e) => setMachineId(e.target.value)}
                  disabled={isPending}
                />
              </Field>

              <Field
                label={t('credentials.proxyConfigLabel')}
                htmlFor="proxyUrl"
                hint={t('credentials.proxyHintAdd')}
              >
                <Input
                  id="proxyUrl"
                  placeholder={t('credentials.proxyUrlPlaceholderAdd')}
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
              {isPending ? t('credentials.addingButton') : t('credentials.addButton')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
