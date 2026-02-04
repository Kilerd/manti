import { createContext, useContext, useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { authAPI } from '@/services/api';
import { toast } from '@/hooks/use-toast';

const AuthContext = createContext({});

// Token expiry check utility
function isTokenExpired(token) {
  if (!token) return true;

  try {
    // Parse JWT token
    const base64Url = token.split('.')[1];
    const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
    const jsonPayload = decodeURIComponent(
      atob(base64)
        .split('')
        .map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
        .join('')
    );

    const payload = JSON.parse(jsonPayload);

    // Check expiration
    if (payload.exp) {
      const currentTime = Date.now() / 1000;
      return payload.exp < currentTime;
    }

    return false;
  } catch (error) {
    console.error('Error parsing token:', error);
    return true;
  }
}

export function AuthProvider({ children }) {
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  // Check token validity on mount and periodically
  useEffect(() => {
    checkAuth();

    // Check token validity every minute
    const interval = setInterval(checkAuth, 60000);

    return () => clearInterval(interval);
  }, []);

  const checkAuth = async () => {
    const token = localStorage.getItem('token');

    if (!token || isTokenExpired(token)) {
      localStorage.removeItem('token');
      setUser(null);
      setLoading(false);
      return;
    }

    try {
      // Validate token with backend
      const response = await authAPI.validateToken();
      setUser(response.data.user);
    } catch (error) {
      // Token is invalid
      localStorage.removeItem('token');
      setUser(null);

      if (window.location.pathname !== '/login' && window.location.pathname !== '/register') {
        toast({
          title: "Session Expired",
          description: "Please login again to continue.",
          variant: "destructive",
        });
        navigate('/login');
      }
    } finally {
      setLoading(false);
    }
  };

  const login = async (credentials) => {
    try {
      const response = await authAPI.login(credentials);
      const { token, user, refreshToken } = response.data;

      localStorage.setItem('token', token);
      if (refreshToken) {
        localStorage.setItem('refreshToken', refreshToken);
      }

      setUser(user);

      toast({
        title: "Login Successful",
        description: "Welcome back!",
      });

      navigate('/dashboard');
      return { success: true };
    } catch (error) {
      const message = error.response?.data?.message || 'Login failed. Please try again.';
      toast({
        title: "Login Failed",
        description: message,
        variant: "destructive",
      });
      return { success: false, error: message };
    }
  };

  const register = async (userData) => {
    try {
      const response = await authAPI.register(userData);
      const { token, user, refreshToken } = response.data;

      localStorage.setItem('token', token);
      if (refreshToken) {
        localStorage.setItem('refreshToken', refreshToken);
      }

      setUser(user);

      toast({
        title: "Registration Successful",
        description: "Welcome to Manti Gateway!",
      });

      navigate('/dashboard');
      return { success: true };
    } catch (error) {
      const message = error.response?.data?.message || 'Registration failed. Please try again.';
      toast({
        title: "Registration Failed",
        description: message,
        variant: "destructive",
      });
      return { success: false, error: message };
    }
  };

  const logout = async () => {
    try {
      await authAPI.logout();
    } catch (error) {
      // Ignore logout errors
    } finally {
      localStorage.removeItem('token');
      localStorage.removeItem('refreshToken');
      setUser(null);

      toast({
        title: "Logged Out",
        description: "You have been successfully logged out.",
      });

      navigate('/login');
    }
  };

  const refreshToken = async () => {
    const refresh = localStorage.getItem('refreshToken');

    if (!refresh) {
      throw new Error('No refresh token available');
    }

    try {
      const response = await authAPI.refreshToken({ refreshToken: refresh });
      const { token } = response.data;

      localStorage.setItem('token', token);
      return token;
    } catch (error) {
      // Refresh failed, need to re-login
      localStorage.removeItem('token');
      localStorage.removeItem('refreshToken');
      setUser(null);
      throw error;
    }
  };

  const value = {
    user,
    loading,
    login,
    register,
    logout,
    refreshToken,
    checkAuth,
    isAuthenticated: !!user,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}