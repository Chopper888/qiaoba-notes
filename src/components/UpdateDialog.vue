<script setup>
import { ref } from 'vue'
import { relaunch } from '@tauri-apps/plugin-process'

const props = defineProps({
  update: { type: Object, required: true }
})
const emit = defineEmits(['close'])

const phase = ref('idle') // idle | downloading | done | error
const progress = ref(0)
const errorMsg = ref('')

async function doUpdate() {
  phase.value = 'downloading'
  progress.value = 0
  try {
    let downloaded = 0
    let total = 0
    await props.update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? 0
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength
        if (total > 0) progress.value = Math.round(downloaded / total * 100)
      } else if (event.event === 'Finished') {
        progress.value = 100
      }
    })
    phase.value = 'done'
    await relaunch()
  } catch (e) {
    phase.value = 'error'
    errorMsg.value = String(e)
  }
}

function dismiss() {
  emit('close')
}
</script>

<template>
  <div class="update-overlay" @click.self="phase === 'idle' && dismiss()">
    <div class="update-dialog">

      <template v-if="phase === 'idle'">
        <div class="update-title">发现新版本 {{ update.version }}</div>
        <div v-if="update.body" class="update-notes">{{ update.body }}</div>
        <div class="update-actions">
          <button class="btn-update-primary" @click="doUpdate">立即更新</button>
          <button class="btn-update-secondary" @click="dismiss">稍后再说</button>
        </div>
      </template>

      <template v-else-if="phase === 'downloading'">
        <div class="update-title">正在下载更新...</div>
        <div class="update-progress-bar">
          <div class="update-progress-fill" :style="{ width: progress + '%' }"></div>
        </div>
        <div class="update-progress-text">{{ progress }}%</div>
      </template>

      <template v-else-if="phase === 'done'">
        <div class="update-title">安装完成，正在重启应用...</div>
      </template>

      <template v-else-if="phase === 'error'">
        <div class="update-title">更新失败</div>
        <div class="update-error-msg">{{ errorMsg }}</div>
        <div class="update-actions">
          <button class="btn-update-secondary" @click="dismiss">关闭</button>
        </div>
      </template>

    </div>
  </div>
</template>

<style scoped>
.update-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
}

.update-dialog {
  background: #fff;
  border-radius: 10px;
  padding: 24px 28px;
  width: 300px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
}

.update-title {
  font-size: 15px;
  font-weight: 600;
  color: #1a1a1a;
  margin-bottom: 10px;
}

.update-notes {
  font-size: 12px;
  color: #666;
  line-height: 1.6;
  margin-bottom: 18px;
  max-height: 80px;
  overflow-y: auto;
  white-space: pre-wrap;
}

.update-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 18px;
}

.btn-update-primary {
  background: #f5a623;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}
.btn-update-primary:hover { background: #e0941a; }

.btn-update-secondary {
  background: #f0f0f0;
  color: #555;
  border: none;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
}
.btn-update-secondary:hover { background: #e0e0e0; }

.update-progress-bar {
  width: 100%;
  height: 6px;
  background: #eee;
  border-radius: 3px;
  overflow: hidden;
  margin: 12px 0 6px;
}

.update-progress-fill {
  height: 100%;
  background: #f5a623;
  border-radius: 3px;
  transition: width 0.2s ease;
}

.update-progress-text {
  font-size: 12px;
  color: #888;
  text-align: center;
}

.update-error-msg {
  font-size: 12px;
  color: #e53e3e;
  line-height: 1.5;
  margin-bottom: 4px;
  word-break: break-all;
}
</style>
