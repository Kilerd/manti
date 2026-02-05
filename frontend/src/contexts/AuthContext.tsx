import {
  createContext,
  useContext,
  useState,
  useEffect,
  type ReactNode,
} from "react";
import { useNavigate } from "react-router-dom";
import { client, type User } from "@/api";
import { toast } from "@/hooks/use-toast";

interface AuthContextValue {
  user: User | null;
  loading: boolean;
  login: (credentials: {
    email: string;
    password: string;
  }) => Promise<{ success: boolean; error?: string }>;
  register: (userData: {
    email: string;
    password: string;
    name: string;
  }) => Promise<{ success: boolean; error?: string }>;
  logout: () => Promise<void>;
  refreshToken: () => Promise<string>;
  checkAuth: () => Promise<void>;
  isAuthenticated: boolean;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function isTokenExpired(token: string): boolean {
  if (!token) return true;

  try {
    const base64Url = token.split(".")[1];
    const base64 = base64Url.replace(/-/g, "+").replace(/_/g, "/");
    const jsonPayload = decodeURIComponent(
      atob(base64)
        .split("")
        .map((c) => "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2))
        .join("")
    );

    const payload = JSON.parse(jsonPayload);

    if (payload.exp) {
      const currentTime = Date.now() / 1000;
      return payload.exp < currentTime;
    }

    return false;
  } catch (error) {
    console.error("Error parsing token:", error);
    return true;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  useEffect(() => {
    checkAuth();

    const interval = setInterval(checkAuth, 60000);

    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const checkAuth = async () => {
    const token = localStorage.getItem("token");

    if (!token || isTokenExpired(token)) {
      localStorage.removeItem("token");
      setUser(null);
      setLoading(false);
      return;
    }

    try {
      const { data, error } = await client.GET("/auth/validate");

      if (error || !data) {
        throw new Error("Validation failed");
      }

      setUser(data.user);
    } catch {
      localStorage.removeItem("token");
      setUser(null);

      if (
        window.location.pathname !== "/login" &&
        window.location.pathname !== "/register"
      ) {
        toast({
          title: "Session Expired",
          description: "Please login again to continue.",
          variant: "destructive",
        });
        navigate("/login");
      }
    } finally {
      setLoading(false);
    }
  };

  const login = async (credentials: { email: string; password: string }) => {
    try {
      const { data, error } = await client.POST("/auth/login", {
        body: credentials,
      });

      if (error || !data) {
        const message =
          (error as { message?: string })?.message ||
          "Login failed. Please try again.";
        toast({
          title: "Login Failed",
          description: message,
          variant: "destructive",
        });
        return { success: false, error: message };
      }

      const { token, user: userData, refreshToken: refresh } = data;

      localStorage.setItem("token", token);
      if (refresh) {
        localStorage.setItem("refreshToken", refresh);
      }

      setUser(userData);

      toast({
        title: "Login Successful",
        description: "Welcome back!",
      });

      navigate("/dashboard");
      return { success: true };
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Login failed. Please try again.";
      toast({
        title: "Login Failed",
        description: message,
        variant: "destructive",
      });
      return { success: false, error: message };
    }
  };

  const register = async (userData: {
    email: string;
    password: string;
    name: string;
  }) => {
    try {
      const { data, error } = await client.POST("/auth/register", {
        body: userData,
      });

      if (error || !data) {
        const message =
          (error as { message?: string })?.message ||
          "Registration failed. Please try again.";
        toast({
          title: "Registration Failed",
          description: message,
          variant: "destructive",
        });
        return { success: false, error: message };
      }

      const { token, user: newUser, refreshToken: refresh } = data;

      localStorage.setItem("token", token);
      if (refresh) {
        localStorage.setItem("refreshToken", refresh);
      }

      setUser(newUser);

      toast({
        title: "Registration Successful",
        description: "Welcome to Manti Gateway!",
      });

      navigate("/dashboard");
      return { success: true };
    } catch (err) {
      const message =
        err instanceof Error
          ? err.message
          : "Registration failed. Please try again.";
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
      await client.POST("/auth/logout");
    } catch {
      // Ignore logout errors
    } finally {
      localStorage.removeItem("token");
      localStorage.removeItem("refreshToken");
      setUser(null);

      toast({
        title: "Logged Out",
        description: "You have been successfully logged out.",
      });

      navigate("/login");
    }
  };

  const refreshToken = async (): Promise<string> => {
    const refresh = localStorage.getItem("refreshToken");

    if (!refresh) {
      throw new Error("No refresh token available");
    }

    try {
      const { data, error } = await client.POST("/auth/refresh", {
        body: { refreshToken: refresh },
      });

      if (error || !data) {
        throw new Error("Refresh failed");
      }

      const { token } = data;

      localStorage.setItem("token", token);
      return token;
    } catch {
      localStorage.removeItem("token");
      localStorage.removeItem("refreshToken");
      setUser(null);
      throw new Error("Token refresh failed");
    }
  };

  const value: AuthContextValue = {
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

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}
