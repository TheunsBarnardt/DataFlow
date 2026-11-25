import type { JsonSchema } from "@dataflow/shared";

/** Shared OpenAPI component schemas mirroring Wealth.Framework.Entities shapes. */
export const wealthSchemas: Record<string, JsonSchema> = {
  MessageResult: {
    type: "object",
    properties: {
      IsSuccessful: { type: "boolean" },
      Message: { type: "string" },
    },
  },
  AuthResult: {
    type: "object",
    properties: {
      IsSuccessful: { type: "boolean" },
      Message: { type: "string" },
      Access_Token: { type: "string" },
      Roles: { type: "array", items: { type: "string" } },
      UserId: { type: "string" },
      UserName: { type: "string" },
    },
  },
  DropdownListItem: {
    type: "object",
    properties: {
      Value: { type: "string" },
      Text: { type: "string" },
    },
  },
  ClientDashboardResult: {
    type: "object",
    properties: {
      TotalValue: { type: "number" },
      Accounts: { type: "array", items: { type: "object" } },
      Currency: { type: "string" },
    },
  },
  PortfolioResult: {
    type: "object",
    properties: {
      AccountNumber: { type: "string" },
      Holdings: { type: "array", items: { type: "object" } },
      TotalMarketValue: { type: "number" },
    },
  },
  TransactionResult: {
    type: "object",
    properties: {
      Transactions: { type: "array", items: { type: "object" } },
    },
  },
  ClientDetailResult: {
    type: "object",
    properties: {
      DynamicsUserId: { type: "string" },
      FirstName: { type: "string" },
      LastName: { type: "string" },
      Email: { type: "string" },
    },
  },
  DocumentResult: {
    type: "object",
    properties: {
      DocId: { type: "string" },
      Title: { type: "string" },
      Date: { type: "string" },
    },
  },
  ForexResult: {
    type: "object",
    properties: {
      Currency: { type: "string" },
      Rate: { type: "number" },
    },
  },
  IndicesResult: {
    type: "object",
    properties: {
      Name: { type: "string" },
      Value: { type: "number" },
      Change: { type: "number" },
    },
  },
  TickerResult: {
    type: "object",
    properties: {
      Code: { type: "string" },
      Price: { type: "number" },
    },
  },
  ReferenceDataResult: {
    type: "object",
    properties: {
      Id: { type: "string" },
      Name: { type: "string" },
    },
  },
  FaqResult: {
    type: "object",
    properties: {
      Id: { type: "integer" },
      Question: { type: "string" },
      Answer: { type: "string" },
      Category: { type: "string" },
    },
  },
  NotificationResult: {
    type: "object",
    properties: {
      Id: { type: "integer" },
      Message: { type: "string" },
      MessageType: { type: "string" },
    },
  },
};

export function ref(name: string): JsonSchema {
  return { $ref: `#/components/schemas/${name}` };
}

export function arrayOf(name: string): JsonSchema {
  return { type: "array", items: ref(name) };
}

export function queryProps(
  props: Record<string, JsonSchema>,
  required: string[] = [],
): JsonSchema {
  return {
    type: "object",
    properties: props,
    required,
  };
}
