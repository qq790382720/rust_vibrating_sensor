<template>
  <div class="config-container">
    <header class="config-header">
      <h1>振动传感器配置</h1>
      <p class="subtitle">系统参数设置</p>
    </header>

    <main class="config-main">
      <form @submit.prevent="handleSave" class="config-form">
        <!-- 基本设置 -->
        <section class="form-section">
          <h2>基本设置</h2>
          <div class="form-grid">
            <div class="form-group">
              <label for="port">监听端口</label>
              <input
                id="port"
                v-model.number="config.port"
                type="number"
                min="1"
                max="65535"
                placeholder="22009"
              />
            </div>
            <div class="form-group">
              <label for="samplingLength">采样长度</label>
              <input
                id="samplingLength"
                v-model.number="config.sampling_length"
                type="number"
                min="1"
                placeholder="100"
              />
            </div>
          </div>
        </section>

        <!-- MQTT设置 -->
        <section class="form-section">
          <h2>MQTT 设置</h2>
          <div class="form-group full-width">
            <label for="mqttUrl">服务器地址</label>
            <input
              id="mqttUrl"
              v-model="config.mqtt_server.url"
              type="text"
              placeholder="127.0.0.1:1883"
            />
          </div>
          <div class="form-grid">
            <div class="form-group">
              <label for="publicTopic">发布主题</label>
              <input
                id="publicTopic"
                v-model="config.mqtt_server.public_topic"
                type="text"
                placeholder="topic/tests"
              />
            </div>
            <div class="form-group">
              <label for="clientId">客户端ID</label>
              <input
                id="clientId"
                v-model="config.mqtt_server.client_id"
                type="text"
                placeholder="scada01"
              />
            </div>
            <div class="form-group">
              <label for="mqttUsername">用户名</label>
              <input
                id="mqttUsername"
                v-model="config.mqtt_server.username"
                type="text"
                placeholder="username"
              />
            </div>
            <div class="form-group">
              <label for="mqttPassword">密码</label>
              <input
                id="mqttPassword"
                v-model="config.mqtt_server.password"
                type="password"
                placeholder="password"
              />
            </div>
          </div>
        </section>

        <!-- 数据上传设置 -->
        <section class="form-section">
          <h2>数据上传设置</h2>
          <div class="form-group full-width">
            <label for="companyId">租户ID (Company ID)</label>
            <input
              id="companyId"
              v-model="config.data_upload.company_id"
              type="text"
              placeholder="cbd9ef26db814b58aa33fb0457eca8af"
            />
          </div>
          <div class="form-grid">
            <div class="form-group">
              <label for="gatewayId">网关ID (Gateway ID)</label>
              <input
                id="gatewayId"
                v-model="config.data_upload.gateway_id"
                type="text"
                placeholder="gfwg"
              />
            </div>
            <div class="form-group">
              <label for="deviceId">设备ID (Device ID)</label>
              <input
                id="deviceId"
                v-model="config.data_upload.device_id"
                type="text"
                placeholder="weldingrobot_d15"
              />
            </div>
          </div>
        </section>

        <!-- 操作按钮 -->
        <div class="form-actions">
          <button type="button" @click="loadConfig" class="btn-secondary">
            <span class="btn-icon">↻</span> 重新加载
          </button>
          <button type="button" @click="handleRestart" class="btn-danger">
            <span class="btn-icon">⏻</span> 重启设备
          </button>
          <button type="submit" class="btn-primary">
            <span class="btn-icon">✓</span> 保存配置
          </button>
        </div>
      </form>
    </main>

    <!-- 提示消息 -->
    <Transition name="fade">
      <div v-if="message.text" :class="['message', message.type]">
        {{ message.text }}
      </div>
    </Transition>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'

const config = ref({
  port: 22009,
  sampling_length: 100,
  mqtt_server: {
    url: '',
    public_topic: '',
    client_id: '',
    username: '',
    password: ''
  },
  data_upload: {
    company_id: '',
    gateway_id: '',
    device_id: ''
  }
})

const message = reactive({ text: '', type: 'success' })

function showMessage(text, type = 'success') {
  message.text = text
  message.type = type
  setTimeout(() => { message.text = '' }, 3000)
}

async function loadConfig() {
  try {
    const response = await fetch('/read-settings')
    if (!response.ok) throw new Error('加载失败')
    const data = await response.json()
    config.value = data
    showMessage('配置加载成功', 'success')
  } catch (error) {
    console.error('加载配置失败:', error)
    showMessage('加载配置失败，请重试', 'error')
  }
}

async function handleSave() {
  try {
    const response = await fetch('/write-settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config.value)
    })
    if (!response.ok) throw new Error('保存失败')
    showMessage('配置保存成功！', 'success')
  } catch (error) {
    console.error('保存配置失败:', error)
    showMessage('保存失败，请重试', 'error')
  }
}

async function handleRestart() {
  if (!confirm('确定要重启边缘设备吗？此操作将导致服务短暂中断！')) return
  try {
    await fetch('/system_restart')
    showMessage('重启指令已发送！', 'success')
  } catch (error) {
    console.error('重启失败:', error)
    showMessage('重启失败，请检查设备连接', 'error')
  }
}

onMounted(() => {
  loadConfig()
})
</script>

<style scoped>
.config-container {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 2rem;
}

.config-header {
  text-align: center;
  margin-bottom: 2rem;
  color: white;
}

.config-header h1 {
  margin: 0;
  font-size: 2rem;
  font-weight: 600;
}

.config-header .subtitle {
  margin: 0.5rem 0 0;
  opacity: 0.9;
}

.config-main {
  max-width: 640px;
  margin: 0 auto;
}

.config-form {
  background: white;
  border-radius: 16px;
  padding: 2rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.form-section {
  margin-bottom: 2rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid #eee;
}

.form-section:last-of-type {
  border-bottom: none;
  margin-bottom: 1.5rem;
  padding-bottom: 0;
}

.form-section h2 {
  margin: 0 0 1rem;
  font-size: 1rem;
  color: #333;
  font-weight: 600;
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

.form-group {
  display: flex;
  flex-direction: column;
}

.form-group.full-width {
  grid-column: 1 / -1;
}

.form-group label {
  font-size: 0.85rem;
  color: #666;
  margin-bottom: 0.4rem;
  font-weight: 500;
}

.form-group input {
  padding: 0.75rem 1rem;
  border: 1px solid #ddd;
  border-radius: 8px;
  font-size: 1rem;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: #667eea;
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.15);
}

.form-actions {
  display: flex;
  gap: 0.75rem;
  justify-content: flex-end;
  margin-top: 1.5rem;
}

button {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.75rem 1.25rem;
  border: none;
  border-radius: 8px;
  font-size: 0.95rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-icon {
  font-size: 1rem;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-secondary {
  background: #f5f5f5;
  color: #333;
}

.btn-secondary:hover {
  background: #eee;
}

.btn-danger {
  background: #dc3545;
  color: white;
}

.btn-danger:hover {
  background: #c82333;
}

.message {
  position: fixed;
  bottom: 2rem;
  left: 50%;
  transform: translateX(-50%);
  padding: 1rem 2rem;
  border-radius: 8px;
  font-weight: 500;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
}

.message.success {
  background: #28a745;
  color: white;
}

.message.error {
  background: #dc3545;
  color: white;
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.3s;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}

@media (max-width: 600px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
  .form-actions {
    flex-direction: column;
  }
  button {
    justify-content: center;
  }
}
</style>
