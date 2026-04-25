import { createRouter, createWebHistory } from 'vue-router'
import Home from '@/views/Home.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/library/live'
    },
    {
      path: '/library/:type',
      name: 'library',
      component: Home
    },
    {
      path: '/player/:mode/:id',
      name: 'player',
      component: () => import('@/views/PlayerPage.vue')
    },
    {
      path: '/subscriptions',
      name: 'subscriptions',
      component: () => import('@/views/Subscriptions.vue')
    },
    {
      path: '/detail/:itemId',
      name: 'detail',
      component: () => import('@/views/VodDetail.vue')
    },
    {
      path: '/detail/hot/:doubanId',
      name: 'HotDetail',
      component: () => import('@/views/HotDetail.vue')
    },
    {
      path: '/vod/:id',
      redirect: to => `/detail/${to.params.id}`
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/Settings.vue')
    }
  ]
})

export default router
