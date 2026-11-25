import { betterAuth } from "better-auth";
import { mongodbAdapter } from "@better-auth/mongo-adapter";
import { apiKey } from "@better-auth/api-key";
import { admin, username } from "better-auth/plugins";
import { getDb } from "../db.js";
import { config } from "../config.js";

export type DataFlowRole =
  | "admin"
  | "editor"
  | "viewer"
  | "api_consumer"
  | "mcp_consumer";

let authInstance: ReturnType<typeof createAuth> | null = null;

function createAuth() {
  return betterAuth({
    appName: "DataFlow",
    baseURL: config.betterAuthUrl,
    secret: config.betterAuthSecret,
    database: mongodbAdapter(getDb()),
    emailAndPassword: {
      enabled: true,
      minPasswordLength: 8,
    },
    user: {
      additionalFields: {
        companyName: {
          type: "string",
          required: false,
          input: true,
        },
        status: {
          type: "string",
          required: false,
          defaultValue: "active",
          input: false,
        },
      },
    },
    plugins: [
      username({
        minUsernameLength: 3,
        maxUsernameLength: 64,
      }),
      admin({
        defaultRole: "editor",
        adminRoles: ["admin"],
      }),
      apiKey({
        defaultPrefix: "bf_",
        enableMetadata: true,
      }),
    ],
    trustedOrigins: [
      config.betterAuthUrl,
      config.publicBaseUrl,
      "http://127.0.0.1:5173",
      "http://localhost:5173",
      "http://127.0.0.1:5174",
      "http://localhost:5174",
      "http://127.0.0.1:8787",
      "http://localhost:8787",
    ],
  });
}

export function getAuth() {
  if (!authInstance) {
    authInstance = createAuth();
  }
  return authInstance;
}

export type Auth = ReturnType<typeof getAuth>;
