import { createRouter, createWebHistory } from 'vue-router'
import Home from '@/views/Home.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home
    },
    {
      path: '/live',
      name: 'live',
      component: () => import('@/views/Live.vue')
    },
    {
      path: '/vod',
      name: 'vod',
      component: () => import('@/views/Vod.vue')
    },
    {
      path: '/player/:type/:id',
      name: 'player',
      component: () => import('@/views/PlayerPage.vue')
    },
    {
      path: '/subscriptions',
      name: 'subscriptions',
      component: () => import('@/views/Subscriptions.vue')
    },
    {
      path: '/vod/:id',
      name: 'vod-detail',
      component: () => import('@/views/VodDetail.vue')
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/Settings.vue')
    }
  ]
})

export default router
