import type { JsonSchema } from "@dataflow/shared";

/** Shared OpenAPI schemas for Sasfin.Dynamics.Api. */
export const dynamicsSchemas: Record<string, JsonSchema> = {
  ClientInformationDTO: {
    type: "object",
    properties: {
      ContactId: { type: "string", format: "uuid" },
      FirstName: { type: "string" },
      LastName: { type: "string" },
      Email: { type: "string" },
      IdentityNumber: { type: "string" },
    },
  },
  SasfinAccountDTO: {
    type: "object",
    properties: {
      Id: { type: "string", format: "uuid" },
      UniqueNumber: { type: "string" },
      ContactId: { type: "string", format: "uuid" },
    },
  },
  ReferenceDataDTO: {
    type: "object",
    properties: {
      Id: { type: "string" },
      Name: { type: "string" },
    },
  },
  AnnotationModelDTO: {
    type: "object",
    properties: {
      Id: { type: "string", format: "uuid" },
      Subject: { type: "string" },
      NoteText: { type: "string" },
    },
  },
  IncidentModelDTO: {
    type: "object",
    properties: {
      Id: { type: "string", format: "uuid" },
      Title: { type: "string" },
      Description: { type: "string" },
    },
  },
};
