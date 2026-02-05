import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
  useNavigate,
} from "react-router-dom";
import { AuthProvider, useAuth } from "@/contexts/AuthContext";
import { Toaster } from "@/components/ui/toaster";
import Layout from "@/components/Layout";
import Login from "@/pages/Login";
import Register from "@/pages/Register";
import Dashboard from "@/pages/Dashboard";
import Models from "@/pages/Models";
import ApiKeys from "@/pages/ApiKeys";
import Usage from "@/pages/Usage";
import Billing from "@/pages/Billing";
import Profile from "@/pages/Profile";
import AdminProviders from "@/pages/admin/Providers";
import AdminUsers from "@/pages/admin/Users";
import { useEffect, type ReactNode } from "react";
import { toast } from "@/hooks/use-toast";

function PrivateRoute({ children }: { children: ReactNode }) {
  const { isAuthenticated, loading } = useAuth();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return isAuthenticated ? children : <Navigate to="/login" />;
}

function AuthListener() {
  const navigate = useNavigate();
  const { logout } = useAuth();

  useEffect(() => {
    const handleAuthLogout = (event: CustomEvent<{ reason?: string }>) => {
      const reason = event.detail?.reason;

      if (reason === "session_expired") {
        toast({
          title: "Session Expired",
          description: "Please login again to continue.",
          variant: "destructive",
        });
      } else if (reason === "refresh_failed") {
        toast({
          title: "Authentication Failed",
          description: "Please login again.",
          variant: "destructive",
        });
      }

      logout();
      navigate("/login");
    };

    window.addEventListener(
      "auth:logout",
      handleAuthLogout as EventListener
    );

    return () => {
      window.removeEventListener(
        "auth:logout",
        handleAuthLogout as EventListener
      );
    };
  }, [navigate, logout]);

  return null;
}

function AppRoutes() {
  return (
    <>
      <AuthListener />
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/register" element={<Register />} />
        <Route
          path="/"
          element={
            <PrivateRoute>
              <Layout />
            </PrivateRoute>
          }
        >
          <Route index element={<Navigate to="/dashboard" />} />
          <Route path="dashboard" element={<Dashboard />} />
          <Route path="models" element={<Models />} />
          <Route path="api-keys" element={<ApiKeys />} />
          <Route path="usage" element={<Usage />} />
          <Route path="billing" element={<Billing />} />
          <Route path="profile" element={<Profile />} />
          <Route path="admin/providers" element={<AdminProviders />} />
          <Route path="admin/users" element={<AdminUsers />} />
        </Route>
      </Routes>
    </>
  );
}

function App() {
  return (
    <Router>
      <AuthProvider>
        <AppRoutes />
        <Toaster />
      </AuthProvider>
    </Router>
  );
}

export default App;
