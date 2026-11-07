<script setup lang="ts">
import { ref } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { profileUrl, useAppStore } from '@/stores/app'
import type { Profile } from '@/shared/api/types'
import SectionCard from '@/shared/ui/SectionCard.vue'
import Icon from '@/shared/ui/Icon.vue'
import ProfileForm from '@/features/profiles/ProfileForm.vue'

const store = useAppStore()
const dialog = useDialog()
const message = useMessage()

const formShow = ref(false)
const editing = ref<Profile | null>(null)

function add() {
  editing.value = null
  formShow.value = true
}

function edit(profile: Profile) {
  editing.value = profile
  formShow.value = true
}

async function makeDefault(profile: Profile) {
  const ok = await store.setDefault(profile.id)
  if (ok) message.success(`已将「${profile.name}」设为默认配置`)
}

function remove(profile: Profile) {
  const active = profile.id === store.state?.activeProfileId
  dialog.warning({
    title: '删除代理配置',
    content: active
      ? `「${profile.name}」正在使用，删除后会自动切换到默认配置。`
      : `确定删除「${profile.name}」？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      const ok = await store.deleteProfile(profile.id)
      if (ok) message.success('配置已删除')
    }
  })
}
</script>

<template>
  <div class="settings-page">
    <div class="page-head">
      <div>
        <h1>代理配置</h1>
        <p>保存多套代理地址，面板中一键切换；切换期间不会先关闭代理。</p>
      </div>
      <div class="page-head-actions">
        <button class="mini-btn primary" type="button" @click="add">
          <Icon name="plus" :size="14" />
          新增
        </button>
      </div>
    </div>

    <SectionCard title="配置列表" :hint="`${store.profiles.length} 套`">
      <div v-if="store.profiles.length === 0" class="empty-state">还没有配置，点击右上角新增一套。</div>
      <div v-for="profile in store.profiles" :key="profile.id" class="profile-item">
        <span class="profile-mark"><Icon name="globe" :size="16" /></span>
        <div class="profile-info">
          <div class="profile-title">
            <strong>{{ profile.name }}</strong>
            <span v-if="profile.id === store.state?.defaultProfileId" class="chip">默认</span>
            <span v-if="profile.id === store.state?.activeProfileId" class="chip accent">使用中</span>
          </div>
          <div class="profile-url">{{ profileUrl(profile) }}</div>
        </div>
        <div class="profile-actions">
          <button
            class="mini-btn"
            type="button"
            :disabled="profile.id === store.state?.defaultProfileId || Boolean(store.busy)"
            @click="makeDefault(profile)"
          >
            设为默认
          </button>
          <button class="icon-btn" type="button" title="编辑" @click="edit(profile)">
            <Icon name="edit" :size="15" />
          </button>
          <button
            class="icon-btn danger"
            type="button"
            title="删除"
            :disabled="profile.id === store.state?.defaultProfileId"
            @click="remove(profile)"
          >
            <Icon name="trash" :size="15" />
          </button>
        </div>
      </div>
    </SectionCard>

    <SectionCard title="说明">
      <div class="note-row">
        <strong>默认配置</strong>
        <p>未指定使用中配置时，快捷键与托盘切换会使用默认配置。默认配置不能删除。</p>
      </div>
      <div class="note-row">
        <strong>无缝切换</strong>
        <p>代理开启时切换配置只改写地址，不会先关闭再开启，连接中断时间更短。</p>
      </div>
    </SectionCard>

    <ProfileForm v-model:show="formShow" :profile="editing" />
  </div>
</template>

<style scoped>
.profile-item {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.profile-item:last-child {
  border-bottom: 0;
}

.profile-mark {
  width: 32px;
  height: 32px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--accent);
}

.profile-info {
  min-width: 0;
  flex: 1;
}

.profile-title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.profile-title strong {
  font-size: 12.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip {
  flex: none;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--surface-3);
  color: var(--muted);
  font-size: 10px;
  font-weight: 600;
}

.chip.accent {
  background: var(--accent-soft);
  color: var(--accent-strong);
}

.chip.subtle {
  background: var(--warn-soft);
  color: var(--warn);
}

.profile-url {
  margin-top: 2px;
  font-family: var(--mono);
  font-size: 10.5px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.profile-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: none;
}

.icon-btn.danger {
  color: var(--danger);
}

.icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.note-row {
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.note-row:last-child {
  border-bottom: 0;
}

.note-row strong {
  display: block;
  font-size: 12.5px;
}

.note-row p {
  margin: 3px 0 0;
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--muted);
}
</style>
