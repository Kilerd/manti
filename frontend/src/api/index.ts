import { Fetcher } from "openapi-typescript-fetch";
import type { paths, operations } from "./schema";

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL || "http://localhost:8080";

if (import.meta.env.PROD && !import.meta.env.VITE_API_BASE_URL) {
  console.warn("VITE_API_BASE_URL is not set in production environment");
}

// Create fetcher instance
const fetcher = Fetcher.for<paths>();

fetcher.configure({
  baseUrl: API_BASE_URL,
  init: {
    headers: {
      "Content-Type": "application/json",
    },
  },
  use: [
    // Auth middleware - add token to requests
    async (url, init, next) => {
      const token = localStorage.getItem("token");
      if (token) {
        init.headers.set("Authorization", `Bearer ${token}`);
      }
      return next(url, init);
    },
  ],
});

// ============ Auth API ============
export const login = fetcher.path("/auth/login").method("post").create();
export const register = fetcher.path("/auth/register").method("post").create();
export const logout = fetcher.path("/auth/logout").method("post").create();
export const validateToken = fetcher.path("/auth/validate").method("get").create();
export const refreshToken = fetcher.path("/auth/refresh").method("post").create();

// ============ User API ============
export const getCurrentUser = fetcher.path("/user/profile").method("get").create();
export const updateProfile = fetcher.path("/user/profile").method("put").create();

// ============ API Keys ============
export const listApiKeys = fetcher.path("/api-keys").method("get").create();
export const createApiKey = fetcher.path("/api-keys").method("post").create();
export const revokeApiKey = fetcher.path("/api-keys/{id}").method("delete").create();

// ============ Usage API ============
export const getUsage = fetcher.path("/usage").method("get").create();
// Note: getUsageStats has duplicate operation name issue in schema,
// using manual fetch as workaround
export async function getUsageStats(): Promise<UsageStats> {
  const response = await fetch(`${API_BASE_URL}/usage/stats`, {
    headers: {
      Authorization: `Bearer ${localStorage.getItem("token")}`,
      "Content-Type": "application/json",
    },
  });
  if (!response.ok) {
    throw new Error("Failed to fetch usage stats");
  }
  return response.json();
}

// ============ Type exports ============
export type User =
  operations["login"]["responses"]["200"]["content"]["application/json"]["user"];

export type ApiKeyListItem =
  operations["list_api_keys"]["responses"]["200"]["content"]["application/json"][number];

export type ApiKeyCreateResponse =
  operations["create_api_key"]["responses"]["200"]["content"]["application/json"];

export type ApiKey = ApiKeyListItem;

// Define UsageStats inline due to schema duplicate operation name issue
export interface UsageStats {
  active_keys: number;
  total_cost: number;
  total_requests: number;
  total_tokens: number;
}

export type UsageRecord =
  operations["get_usage"]["responses"]["200"]["content"]["application/json"][number];

// Re-export paths
export type { paths, operations };

export { API_BASE_URL };
