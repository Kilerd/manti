import axios from 'axios';
import { toast } from '@/hooks/use-toast';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';

// Validate environment configuration in production
if (import.meta.env.PROD && !import.meta.env.VITE_API_BASE_URL) {
  console.warn('VITE_API_BASE_URL is not set in production environment');
}

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Track if we're currently refreshing the token
let isRefreshing = false;
let failedQueue = [];

const processQueue = (error, token = null) => {
  failedQueue.forEach(prom => {
    if (error) {
      prom.reject(error);
    } else {
      prom.resolve(token);
    }
  });

  failedQueue = [];
};

// Add token to requests if it exists
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Handle auth errors and token refresh
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;

    // Handle 401 Unauthorized
    if (error.response?.status === 401 && !originalRequest._retry) {
      if (isRefreshing) {
        // If already refreshing, queue this request
        return new Promise((resolve, reject) => {
          failedQueue.push({ resolve, reject });
        }).then(token => {
          originalRequest.headers.Authorization = `Bearer ${token}`;
          return api(originalRequest);
        }).catch(err => {
          return Promise.reject(err);
        });
      }

      originalRequest._retry = true;
      isRefreshing = true;

      const refreshToken = localStorage.getItem('refreshToken');

      if (!refreshToken) {
        // No refresh token, need to login
        localStorage.removeItem('token');
        localStorage.removeItem('refreshToken');

        // Use a custom event to notify the app about auth failure
        window.dispatchEvent(new CustomEvent('auth:logout', {
          detail: { reason: 'session_expired' }
        }));

        return Promise.reject(error);
      }

      try {
        const response = await axios.post(`${API_BASE_URL}/auth/refresh`, {
          refreshToken
        });

        const { token } = response.data;
        localStorage.setItem('token', token);

        processQueue(null, token);

        originalRequest.headers.Authorization = `Bearer ${token}`;
        return api(originalRequest);
      } catch (refreshError) {
        processQueue(refreshError, null);

        localStorage.removeItem('token');
        localStorage.removeItem('refreshToken');

        // Dispatch logout event
        window.dispatchEvent(new CustomEvent('auth:logout', {
          detail: { reason: 'refresh_failed' }
        }));

        return Promise.reject(refreshError);
      } finally {
        isRefreshing = false;
      }
    }

    return Promise.reject(error);
  }
);

// Auth APIs
export const authAPI = {
  login: (credentials) => api.post('/auth/login', credentials),
  register: (userData) => api.post('/auth/register', userData),
  logout: () => api.post('/auth/logout'),
  validateToken: () => api.get('/auth/validate'),
  refreshToken: (data) => api.post('/auth/refresh', data),
};

// API Key management
export const apiKeyAPI = {
  list: () => api.get('/api-keys'),
  create: (data) => api.post('/api-keys', data),
  revoke: (keyId) => api.delete(`/api-keys/${keyId}`),
};

// Usage statistics
export const usageAPI = {
  getStats: () => api.get('/usage/stats'),
  getHistory: (params) => api.get('/usage/history', { params }),
};

// User profile
export const userAPI = {
  getProfile: () => api.get('/user/profile'),
  updateProfile: (data) => api.put('/user/profile', data),
};

export default api;