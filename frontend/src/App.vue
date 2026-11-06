<script setup lang="ts">
import { computed } from 'vue'
import {
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  darkTheme,
  dateZhCN,
  zhCN,
  type GlobalThemeOverrides
} from 'naive-ui'
import { useTheme } from '@/shared/theme/useTheme'
import AppBody from './AppBody.vue'

const { resolved, palette } = useTheme()
const theme = computed(() => (resolved.value.theme === 'dark' ? darkTheme : null))

const overrides = computed<GlobalThemeOverrides>(() => {
  const p = palette.value
  return {
    common: {
      primaryColor: p.accent,
      primaryColorHover: p.accentStrong,
      primaryColorPressed: p.accentStrong,
      primaryColorSuppl: p.accent,
      infoColor: p.accent,
      successColor: p.success,
      warningColor: p.warn,
      errorColor: p.danger,
      textColorBase: p.text,
      textColor1: p.text,
      textColor2: p.text2,
      textColor3: p.muted,
      bodyColor: p.bg,
      cardColor: p.surface,
      modalColor: p.surface,
      popoverColor: p.surface,
      borderColor: p.border,
      dividerColor: p.border,
      borderRadius: p.radiusMd,
      borderRadiusSmall: p.radiusSm,
      fontFamily: p.font,
      fontSize: '13px',
      fontSizeSmall: '12px',
      placeholderColor: p.muted,
      iconColor: p.muted,
      hoverColor: p.surface2
    },
    Button: {
      textColorPrimary: p.onAccent,
      textColorHoverPrimary: p.onAccent,
      textColorPressedPrimary: p.onAccent,
      textColorFocusPrimary: p.onAccent,
      textColorGhostPrimary: p.accent,
      textColorGhostHoverPrimary: p.accentStrong,
      textColorTextPrimary: p.accent,
      textColorTextHoverPrimary: p.accentStrong
    },
    Input: {
      color: p.surface2,
      colorFocus: p.surface,
      textColor: p.text,
      placeholderColor: p.muted,
      caretColor: p.accent,
      border: `1px solid ${p.controlBorder}`,
      borderHover: `1px solid ${p.accent}`,
      borderFocus: `1px solid ${p.accent}`,
      boxShadowFocus: `0 0 0 3px ${p.accentSoft}`,
      borderError: `1px solid ${p.danger}`,
      borderHoverError: `1px solid ${p.danger}`,
      borderFocusError: `1px solid ${p.danger}`,
      boxShadowFocusError: `0 0 0 3px color-mix(in srgb, ${p.danger} 16%, transparent)`
    },
    Dialog: {
      color: p.surface,
      borderRadius: p.radiusLg,
      titleTextColor: p.text,
      textColor: p.text2
    },
    Message: {
      color: p.surface,
      textColor: p.text,
      borderRadius: p.radiusSm,
      boxShadow: '0 10px 30px -14px rgba(0, 0, 0, .45)'
    }
  }
})
</script>

<template>
  <n-config-provider :theme="theme" :theme-overrides="overrides" :locale="zhCN" :date-locale="dateZhCN">
    <n-message-provider placement="bottom" :max="3">
      <n-dialog-provider>
        <AppBody />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
