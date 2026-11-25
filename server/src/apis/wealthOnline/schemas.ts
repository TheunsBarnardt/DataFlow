import type { JsonSchema } from "@dataflow/shared";

/** Shared OpenAPI schemas for Sasfin.Wealth.Online.Api. */
export const onlineSchemas: Record<string, JsonSchema> = {
  ApiResponse: {
    type: "object",
    properties: {
      IsSuccessful: { type: "boolean" },
      Message: { type: "string" },
      Data: { type: "object" },
    },
  },
  AuthResult: {
    type: "object",
    properties: {
      AccessToken: { type: "string" },
      RefreshToken: { type: "string" },
      UserId: { type: "string" },
      UserName: { type: "string" },
      Roles: { type: "array", items: { type: "string" } },
    },
  },
  AuthTokens: {
    type: "object",
    properties: {
      AccessToken: { type: "string" },
      RefreshToken: { type: "string" },
    },
  },
  ReferenceDataDTO: {
    type: "object",
    properties: {
      Id: { type: "string" },
      Name: { type: "string" },
    },
  },
  CaseViewModel: {
    type: "object",
    properties: {
      Id: { type: "string" },
      Subject: { type: "string" },
      Status: { type: "string" },
    },
  },
};
