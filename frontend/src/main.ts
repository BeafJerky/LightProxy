import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { initTheme } from '@/shared/theme/useTheme'
import './styles/global.css'

initTheme()

createApp(App).use(createPinia()).mount('#app')
