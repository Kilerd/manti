import createClient from "openapi-fetch";
import type {
  paths,
  User,
  UserProfile,
  ApiKey,
  UsageStats,
  UsageRecord,
  UsageSummary,
} from "./schema";

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL || "http://localhost:8080";

if (import.meta.env.PROD && !import.meta.env.VITE_API_BASE_URL) {
  console.warn("VITE_API_BASE_URL is not set in production environment");
}

// Token refresh state
let isRefreshing = false;
interface QueueItem {
  resolve: (token: string) => void;
  reject: (error: Error) => void;
}
let failedQueue: QueueItem[] = [];

function processQueue(error: Error | null, token: string | null = null) {
  failedQueue.forEach((prom) => {
    if (error) {
      prom.reject(error);
    } else if (token) {
      prom.resolve(token);
    }
  });
  failedQueue = [];
}

// Create the base client
const baseClient = createClient<paths>({
  baseUrl: API_BASE_URL,
  headers: {
    "Content-Type": "application/json",
  },
});

// Add auth middleware
baseClient.use({
  async onRequest({ request }) {
    const token = localStorage.getItem("token");
    if (token) {
      request.headers.set("Authorization", `Bearer ${token}`);
    }
    return request;
  },
  async onResponse({ response, request }) {
    if (response.status === 401 && !request.headers.get("X-Retry")) {
      // Handle token refresh
      if (isRefreshing) {
        return new Promise<Response>((resolve, reject) => {
          failedQueue.push({
            resolve: async (token: string) => {
              const newRequest = request.clone();
              newRequest.headers.set("Authorization", `Bearer ${token}`);
              newRequest.headers.set("X-Retry", "true");
              try {
                const retryResponse = await fetch(newRequest);
                resolve(retryResponse);
              } catch (err) {
                reject(err as Error);
              }
            },
            reject,
          });
        });
      }

      isRefreshing = true;

      const refreshToken = localStorage.getItem("refreshToken");
      if (!refreshToken) {
        localStorage.removeItem("token");
        localStorage.removeItem("refreshToken");
        window.dispatchEvent(
          new CustomEvent("auth:logout", {
            detail: { reason: "session_expired" },
          })
        );
        isRefreshing = false;
        return response;
      }

      try {
        const refreshResponse = await fetch(`${API_BASE_URL}/auth/refresh`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ refreshToken }),
        });

        if (!refreshResponse.ok) {
          throw new Error("Refresh failed");
        }

        const data = await refreshResponse.json();
        const newToken = data.token as string;
        localStorage.setItem("token", newToken);

        processQueue(null, newToken);

        // Retry original request
        const newRequest = request.clone();
        newRequest.headers.set("Authorization", `Bearer ${newToken}`);
        newRequest.headers.set("X-Retry", "true");
        return fetch(newRequest);
      } catch (err) {
        processQueue(err as Error, null);
        localStorage.removeItem("token");
        localStorage.removeItem("refreshToken");
        window.dispatchEvent(
          new CustomEvent("auth:logout", {
            detail: { reason: "refresh_failed" },
          })
        );
        return response;
      } finally {
        isRefreshing = false;
      }
    }
    return response;
  },
});

// Export typed client
export const client = baseClient;

// Re-export types
export type {
  paths,
  User,
  UserProfile,
  ApiKey,
  UsageStats,
  UsageRecord,
  UsageSummary,
};

// Helper types for API responses
export type ApiResponse<T> = {
  data?: T;
  error?: { message?: string };
  response: Response;
};

export type AuthLoginResponse =
  paths["/auth/login"]["post"]["responses"]["200"]["content"]["application/json"];
export type AuthRegisterResponse =
  paths["/auth/register"]["post"]["responses"]["200"]["content"]["application/json"];
export type AuthValidateResponse =
  paths["/auth/validate"]["get"]["responses"]["200"]["content"]["application/json"];

export type ApiKeyListResponse =
  paths["/api-keys"]["get"]["responses"]["200"]["content"]["application/json"];
export type ApiKeyCreateResponse =
  paths["/api-keys"]["post"]["responses"]["200"]["content"]["application/json"];

export type UsageHistoryResponse =
  paths["/usage/history"]["get"]["responses"]["200"]["content"]["application/json"];

export { API_BASE_URL };
