import type { ApiAuthMode, ApiHttpMethod, JsonSchema } from "@dataflow/shared";
import { arrayOf, queryProps, ref } from "./schemas.js";

export interface WealthEndpointDef {
  controller: string;
  action: string;
  method: ApiHttpMethod;
  auth: ApiAuthMode;
  summary: string;
  /** Special local implementation key */
  impl?: "serviceAuthenticate" | "tokenAuthenticate" | "proxy";
  requestQuerySchema?: JsonSchema;
  requestBodySchema?: JsonSchema;
  responseSchema?: JsonSchema;
}

function ep(
  controller: string,
  action: string,
  method: ApiHttpMethod,
  auth: ApiAuthMode,
  summary: string,
  extra: Partial<WealthEndpointDef> = {},
): WealthEndpointDef {
  return {
    controller,
    action,
    method,
    auth,
    summary,
    impl: "proxy",
    ...extra,
  };
}

/** Full Sasfin.Wealth.Api controller/action surface mirrored as DataFlow flows. */
export const WEALTH_ENDPOINTS: WealthEndpointDef[] = [
  // —— Auth ——
  ep("Auth", "ServiceAuthenticate", "POST", "none", "Service client credentials auth", {
    impl: "serviceAuthenticate",
    requestQuerySchema: queryProps(
      { clientId: { type: "string" }, clientSecret: { type: "string" } },
      ["clientId", "clientSecret"],
    ),
    responseSchema: ref("MessageResult"),
  }),
  ep("Auth", "TokenAuthenticate", "POST", "none", "Exchange token for JWT", {
    impl: "tokenAuthenticate",
    requestQuerySchema: queryProps({ token: { type: "string" } }, ["token"]),
    responseSchema: ref("AuthResult"),
  }),
  ep("Auth", "ActivateUserAccountAsync", "POST", "none", "Activate user account", {
    requestQuerySchema: queryProps({ userId: { type: "string", format: "uuid" } }, ["userId"]),
    responseSchema: { type: "integer" },
  }),
  ep("Auth", "LinkedAccountNumbersAsync", "GET", "jwt", "Linked account numbers", {
    requestQuerySchema: queryProps({ dynamicsUserId: { type: "string", format: "uuid" } }, ["dynamicsUserId"]),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Auth", "LinkedAccountNumbersForDocumentsAsync", "GET", "jwt", "Linked accounts for documents", {
    requestQuerySchema: queryProps({ dynamicsUserId: { type: "string", format: "uuid" } }, ["dynamicsUserId"]),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Auth", "LinkedCrmAccountsAsync", "GET", "jwt", "Linked CRM accounts", {
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Auth", "ForgotPasswordAsync", "POST", "none", "Forgot password", {
    requestQuerySchema: queryProps({ username: { type: "string" } }, ["username"]),
    responseSchema: ref("MessageResult"),
  }),
  ep("Auth", "ForgotUsernameAsync", "POST", "none", "Forgot username", {
    requestQuerySchema: queryProps({ identityNumber: { type: "string" } }, ["identityNumber"]),
    responseSchema: ref("MessageResult"),
  }),
  ep("Auth", "ChangePasswordAsync", "GET", "jwt", "Change password", {
    requestQuerySchema: queryProps(
      { userId: { type: "string", format: "uuid" }, password: { type: "string" } },
      ["userId", "password"],
    ),
    responseSchema: { type: "boolean" },
  }),
  ep("Auth", "ResetPasswordAsync", "POST", "none", "Reset password", {
    requestQuerySchema: queryProps({
      username: { type: "string" },
      otp: { type: "string" },
      secret: { type: "string" },
      tempPass: { type: "string" },
    }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Auth", "UsersByEMailAsync", "GET", "none", "Users by email", {
    requestQuerySchema: queryProps({ email: { type: "string" } }, ["email"]),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Auth", "DefaultDynamicsUserIdAsync", "GET", "none", "Default Dynamics user id", {
    requestQuerySchema: queryProps({ aspUserId: { type: "string" } }, ["aspUserId"]),
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Auth", "RegisterIFARequestAsync", "POST", "none", "Register IFA request", {
    requestBodySchema: queryProps({ UserName: { type: "string" }, Password: { type: "string" }, FirstName: { type: "string" }, Surname: { type: "string" }, EMail: { type: "string" }, PhoneNumber: { type: "string" }, CrmNumber: { type: "string" }, IdNumber: { type: "string" }, SasfinAccountNumber: { type: "string" }, Consent: { type: "boolean" } }),
    responseSchema: { type: "object" },
  }),
  ep("Auth", "InvidualRelationsAsync", "GET", "none", "Individual relations", {
    requestQuerySchema: queryProps({ input: { type: "string" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Auth", "CheckIfCurrentUserIsPrimaryAccountHolderAsync", "GET", "none", "Is primary account holder", {
    requestQuerySchema: queryProps({
      aspNetUserId: { type: "string" },
      dynamicsUserId: { type: "string", format: "uuid" },
    }),
    responseSchema: { type: "boolean" },
  }),
  ep("Auth", "ElectiveEventAccountsAsync", "GET", "none", "Elective event accounts", {
    requestQuerySchema: queryProps({
      dynamicsUserId: { type: "string", format: "uuid" },
      roleId: { type: "string" },
    }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Auth", "UpdateUserEmailAsync", "POST", "none", "Update user email", {
    requestQuerySchema: queryProps({
      dynamicsUserId: { type: "string", format: "uuid" },
      aspUserId: { type: "string" },
    }),
    responseSchema: ref("MessageResult"),
  }),

  // —— Portfolio ——
  ep("Portfolio", "DashboardAsync", "GET", "jwt", "Client dashboard", {
    requestQuerySchema: queryProps(
      {
        dynamicsUserId: { type: "string", format: "uuid" },
        crmNumber: { type: "string" },
      },
      ["dynamicsUserId", "crmNumber"],
    ),
    responseSchema: ref("ClientDashboardResult"),
  }),
  ep("Portfolio", "PortfolioAsync", "GET", "jwt", "Portfolio holdings", {
    requestQuerySchema: queryProps({
      accountNumber: { type: "string" },
      date: { type: "string", format: "date-time" },
      crmNumber: { type: "string" },
    }),
    responseSchema: ref("PortfolioResult"),
  }),
  ep("Portfolio", "DMWealthPortfolioAsync", "GET", "jwt", "DM Wealth portfolio", {
    requestQuerySchema: queryProps({
      account: { type: "string" },
      currency: { type: "string" },
      date: { type: "string", format: "date-time" },
      externalAccount: { type: "string" },
    }),
    responseSchema: ref("PortfolioResult"),
  }),
  ep("Portfolio", "TransactionAsync", "GET", "jwt", "Portfolio transactions", {
    requestQuerySchema: queryProps({
      accountNumber: { type: "string" },
      startDate: { type: "string", format: "date-time" },
      endDate: { type: "string", format: "date-time" },
      crmNumber: { type: "string" },
    }),
    responseSchema: ref("TransactionResult"),
  }),
  ep("Portfolio", "IsShowPortfolioTab", "GET", "jwt", "Show portfolio tab?", {
    requestQuerySchema: queryProps({ crmnumber: { type: "string" } }),
    responseSchema: { type: "object" },
  }),
  ep("Portfolio", "PortfolioExportAsync", "GET", "jwt", "Export portfolio", {
    responseSchema: { type: "string", format: "byte" },
  }),
  ep("Portfolio", "UpdateInstrumentsReferenceDataAsync", "POST", "jwt", "Update instruments reference data", {
    responseSchema: ref("MessageResult"),
  }),
  ep("Portfolio", "GetBankListAsync", "GET", "jwt", "Bank list", {
    requestQuerySchema: queryProps({
      provider: { type: "string" },
      currency: { type: "string" },
      offshoreallowance: { type: "string" },
      productDynamics: { type: "string" },
    }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Portfolio", "GetBankDetailsAsync", "GET", "jwt", "Bank details", {
    requestQuerySchema: queryProps({
      provider: { type: "string" },
      currency: { type: "string" },
      bank: { type: "string" },
      productDynamics: { type: "string" },
    }),
    responseSchema: { type: "object" },
  }),
  ep("Portfolio", "GetSasfinLinkedAccountsAsync", "GET", "jwt", "Sasfin linked accounts", {
    requestQuerySchema: queryProps({
      dynamicsUserId: { type: "string", format: "uuid" },
      crmNumber: { type: "string" },
    }),
    responseSchema: arrayOf("DropdownListItem"),
  }),

  // —— Market ——
  ep("Market", "AddWatchlistAsync", "POST", "jwt", "Add watchlist", {
    requestQuerySchema: queryProps({ aspUserId: { type: "string" }, description: { type: "string" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Market", "AddWatchlistSharesAsync", "POST", "jwt", "Add watchlist shares", {
    requestQuerySchema: queryProps({
      watchListId: { type: "integer" },
      instrumentCode: { type: "string" },
    }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Market", "ForexAsync", "GET", "none", "Forex rates", {
    responseSchema: arrayOf("ForexResult"),
  }),
  ep("Market", "IndicesAsync", "GET", "none", "Market indices", {
    requestQuerySchema: queryProps({ date: { type: "string", format: "date-time" } }),
    responseSchema: arrayOf("IndicesResult"),
  }),
  ep("Market", "InstrumentLookupAsync", "GET", "none", "Instrument lookup", {
    requestQuerySchema: queryProps({ lookup: { type: "string" } }),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Market", "RemoveWatchlistAsync", "DELETE", "jwt", "Remove watchlist", {
    requestQuerySchema: queryProps({ watchListId: { type: "integer" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Market", "RemoveWatchlistSharesAsync", "DELETE", "jwt", "Remove watchlist shares", {
    requestQuerySchema: queryProps({
      watchListId: { type: "integer" },
      instrumentCode: { type: "string" },
    }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Market", "UserWatchlistsAsync", "GET", "jwt", "User watchlists", {
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Market", "WatchlistSharesAsync", "POST", "jwt", "Watchlist shares", {
    requestQuerySchema: queryProps({ watchlistId: { type: "integer" } }),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Market", "TickerAsync", "GET", "none", "Market ticker", {
    responseSchema: arrayOf("TickerResult"),
  }),

  // —— Crm ——
  ep("Crm", "ClientDetailAsync", "GET", "jwt", "Client detail", {
    requestQuerySchema: queryProps({ dynamicsUserId: { type: "string", format: "uuid" } }),
    responseSchema: ref("ClientDetailResult"),
  }),
  ep("Crm", "LinkPortfolioAsync", "POST", "jwt", "Link portfolio", {
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Crm", "UpdateClientDetailAsync", "POST", "jwt", "Update client detail", {
    requestBodySchema: queryProps({
      DynamicsUserId: { type: "string" },
      FirstName: { type: "string" },
      LastName: { type: "string" },
      Email: { type: "string" },
    }),
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Crm", "UpdateSubscriptionsAsync", "POST", "jwt", "Update subscriptions", {
    requestBodySchema: queryProps({ Initials: { type: "string" }, Title: { type: "string" }, IdNumber: { type: "string" }, FullName: { type: "string" }, MobilePhone: { type: "string" }, MobilePhone2: { type: "string" }, EmailAddress: { type: "string" }, AccountNumber: { type: "string" }, _sasfinAccount: { type: "string", format: "uuid" }, ProductName: { type: "string" }, Category: { type: "string" }, Frequency: { type: "string" }, Subscribed: { type: "boolean" }, SasfinSubscriptions: { type: "array" }, SasfinAccounts: { type: "array" } }),
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Crm", "IndividualByCrmNo", "GET", "jwt", "Individual by CRM number", {
    requestQuerySchema: queryProps({ crmNo: { type: "string" } }),
    responseSchema: { type: "object" },
  }),
  ep("Crm", "LoadSasfinSubscription", "GET", "jwt", "Load Sasfin subscription", {
    requestQuerySchema: queryProps({ clientId: { type: "string", format: "uuid" } }),
    responseSchema: { type: "object" },
  }),
  ep("Crm", "GetAllSasfinSubscriptionAsync", "GET", "jwt", "All Sasfin subscriptions", {
    responseSchema: { type: "object" },
  }),
  ep("Crm", "GetSasfinAccounts", "GET", "jwt", "Sasfin accounts", {
    requestQuerySchema: queryProps({ clientId: { type: "string", format: "uuid" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Crm", "CreateSasfinSubscriptionAsync", "POST", "jwt", "Create Sasfin subscription", {
    requestBodySchema: queryProps({ Initials: { type: "string" }, Title: { type: "string" }, IdNumber: { type: "string" }, FullName: { type: "string" }, MobilePhone: { type: "string" }, MobilePhone2: { type: "string" }, EmailAddress: { type: "string" }, AccountNumber: { type: "string" }, _sasfinAccount: { type: "string", format: "uuid" }, ProductName: { type: "string" }, Category: { type: "string" }, Frequency: { type: "string" }, Subscribed: { type: "boolean" }, SasfinSubscriptions: { type: "array" }, SasfinAccounts: { type: "array" } }),
    responseSchema: { type: "string", format: "uuid" },
  }),

  // —— Document ——
  ep("Document", "ClientDocumentsAsync", "GET", "jwt", "Client documents", {
    requestQuerySchema: queryProps({
      dynamicUserID: { type: "string", format: "uuid" },
      accountNumber: { type: "string" },
      startDate: { type: "string", format: "date-time" },
      endDate: { type: "string", format: "date-time" },
    }),
    responseSchema: arrayOf("DocumentResult"),
  }),
  ep("Document", "ExportAsync", "GET", "jwt", "Export document", {
    requestQuerySchema: queryProps({
      docId: { type: "string", format: "uuid" },
      title: { type: "string" },
    }),
    responseSchema: { type: "object" },
  }),
  ep("Document", "RegenerateOneTimeInfoslipsURL", "GET", "jwt", "Regenerate infoslips URL", {
    responseSchema: ref("MessageResult"),
  }),
  ep("Document", "GetDocumentFolders", "GET", "jwt", "Document folders", {
    requestQuerySchema: queryProps({ UserCrmId: { type: "string", format: "uuid" } }),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Document", "DeleteDocument", "GET", "jwt", "Delete document", {
    requestQuerySchema: queryProps({ docId: { type: "string", format: "uuid" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Document", "GetClientDocumentData", "GET", "jwt", "Client document data", {
    requestQuerySchema: queryProps({ docId: { type: "string", format: "uuid" } }),
    responseSchema: { type: "object" },
  }),
  ep("Document", "GetDynamicsCategories", "GET", "jwt", "Dynamics categories", {
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Document", "GetClientDocumentsByFolder", "GET", "jwt", "Documents by folder", {
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Document", "UploadClientDocumentAsync", "POST", "jwt", "Upload client document", {
    requestBodySchema: queryProps({ DocumentId: { type: "string", format: "uuid" }, UploadToCRMUser: { type: "string", format: "uuid" }, DocumentName: { type: "string" }, CreateDate: { type: "string" }, CreatedBy: { type: "string" }, ExpiryDate: { type: "string" }, DynamicsClassificationTypeString: { type: "string" }, DynamicsClassificationTypeInt: { type: "integer" }, DocumentSizeInBytes: { type: "integer" }, ShareWithSasfin: { type: "boolean" }, IsActive: { type: "boolean" }, DocumentFolderId: { type: "integer" }, FreeText: { type: "string" }, DocumentData: { type: "array" }, DocumentExtention: { type: "string" }, ContentType: { type: "string" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Document", "GetIdorPassportNumberAsync", "GET", "jwt", "ID or passport number", {
    requestQuerySchema: queryProps({ dynamicUserID: { type: "string", format: "uuid" } }),
    responseSchema: { type: "object" },
  }),
  ep("Document", "GetSharePointDocumnetAsync", "GET", "jwt", "SharePoint document", {
    responseSchema: ref("MessageResult"),
  }),
  ep("Document", "GetGeneralDocumentsAsync", "GET", "jwt", "General documents", {
    requestQuerySchema: queryProps({ dynamicUserID: { type: "string" } }),
    responseSchema: arrayOf("DocumentResult"),
  }),

  // —— Manage ——
  ep("Manage", "GetEmailLog", "GET", "jwt", "Email log", {
    requestQuerySchema: queryProps({
      startDate: { type: "string", format: "date-time" },
      endDate: { type: "string", format: "date-time" },
    }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Manage", "CurrentPortfolioProvider", "GET", "jwt", "Current portfolio provider", {
    responseSchema: { type: "object" },
  }),
  ep("Manage", "CurrentMarketDataProvider", "GET", "jwt", "Current market data provider", {
    responseSchema: { type: "object" },
  }),
  ep("Manage", "CheckIfCacheAllDynamics", "GET", "jwt", "Cache all Dynamics check", {
    responseSchema: ref("MessageResult"),
  }),

  // —— ReferenceData ——
  ep("ReferenceData", "CountryAsync", "GET", "jwt", "Countries", {
    responseSchema: arrayOf("ReferenceDataResult"),
  }),
  ep("ReferenceData", "TitlesAsync", "GET", "jwt", "Titles", {
    responseSchema: arrayOf("ReferenceDataResult"),
  }),

  // —— Admin ——
  ep("Admin", "Faq", "GET", "jwt", "FAQ list", { responseSchema: arrayOf("FaqResult") }),
  ep("Admin", "UpdateFaq", "POST", "jwt", "Update FAQ", {
    requestBodySchema: queryProps({
      Id: { type: "integer" },
      Question: { type: "string" },
      Answer: { type: "string" },
      Category: { type: "string" },
      IsActive: { type: "boolean" },
    }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "AddFaq", "POST", "jwt", "Add FAQ", {
    requestBodySchema: queryProps({
      Question: { type: "string" },
      Answer: { type: "string" },
      Sequence: { type: "integer" },
      IsActive: { type: "boolean" },
      Category: { type: "string" },
      EditorAnswer: { type: "string" },
    }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "Notifications", "GET", "none", "Notifications", {
    responseSchema: arrayOf("NotificationResult"),
  }),
  ep("Admin", "UpdateNotification", "POST", "jwt", "Update notification", {
    requestBodySchema: queryProps({ Id: { type: "integer" }, Description: { type: "string" }, MessageType: { type: "string" }, Priority: { type: "integer" }, IsActive: { type: "boolean" } }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "AddNotification", "POST", "jwt", "Add notification", {
    requestBodySchema: queryProps({ Description: { type: "string" }, EditorDescription: { type: "string" }, MessageType: { type: "string" }, Priority: { type: "integer" }, Popup: { type: "boolean" } }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "DeleteNotification", "DELETE", "jwt", "Delete notification", {
    requestQuerySchema: queryProps({ id: { type: "integer" } }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "UserActivityLogsAsync", "GET", "jwt", "User activity logs", {
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "InsertUserActivityAsync", "POST", "jwt", "Insert user activity", {
    requestBodySchema: queryProps({ AspNetUserId: { type: "string", format: "uuid" }, Activity: { type: "string" } }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "RegistrationReportAsync", "GET", "jwt", "Registration report", {
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "SystemRolesAsync", "GET", "jwt", "System roles", {
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "SystemUserAsync", "GET", "jwt", "System users", {
    requestQuerySchema: queryProps({ inputSearch: { type: "string" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "AvailableUserRolesListAsync", "GET", "jwt", "Available user roles", {
    requestQuerySchema: queryProps({ userId: { type: "string" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "CurrentUserRolesList", "GET", "jwt", "Current user roles", {
    requestQuerySchema: queryProps({ userId: { type: "string" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "AddUserRoles", "POST", "jwt", "Add user roles", {
    requestBodySchema: queryProps({ userId: { type: "string" }, roleId: { type: "string" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Admin", "DeleteUserRoles", "DELETE", "jwt", "Delete user roles", {
    requestBodySchema: queryProps({ userId: { type: "string" }, roleId: { type: "string" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Admin", "Edituser", "POST", "jwt", "Edit user", {
    requestBodySchema: queryProps({ Id: { type: "string" }, Email: { type: "string" }, UserName: { type: "string" }, IsActive: { type: "boolean" } }),
    responseSchema: ref("MessageResult"),
  }),
  ep("Admin", "GetUserByID", "GET", "jwt", "Get user by ID", {
    requestQuerySchema: queryProps({ userID: { type: "string" } }),
    responseSchema: { type: "object" },
  }),
  ep("Admin", "SystemUserByEmailAsync", "GET", "jwt", "System user by email", {
    requestQuerySchema: queryProps({ email: { type: "string" } }),
    responseSchema: { type: "object" },
  }),
  ep("Admin", "LinkedPortfoliosAsync", "GET", "jwt", "Linked portfolios", {
    requestQuerySchema: queryProps({ userid: { type: "string", format: "uuid" } }),
    responseSchema: arrayOf("DropdownListItem"),
  }),
  ep("Admin", "DelinkProfileAsync", "GET", "jwt", "Delink profile", {
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Admin", "LinkProfileAsync", "POST", "jwt", "Link profile", {
    requestBodySchema: queryProps({ aspUserId: { type: "string", format: "uuid" }, dynamicsUserId: { type: "string", format: "uuid" }, userName: { type: "string" } }),
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Admin", "SetToDefaultProfileAsync", "GET", "jwt", "Set default profile", {
    responseSchema: { type: "string", format: "uuid" },
  }),
  ep("Admin", "getWebsiteFaqByCategory", "GET", "none", "Website FAQ by category", {
    requestQuerySchema: queryProps({ category: { type: "string" } }),
    responseSchema: arrayOf("FaqResult"),
  }),
  ep("Admin", "getAnswerByQuestionId", "GET", "none", "FAQ answer by question id", {
    requestQuerySchema: queryProps({ questionId: { type: "integer" } }),
    responseSchema: arrayOf("FaqResult"),
  }),
  ep("Admin", "UserNotification", "GET", "jwt", "User notifications", {
    requestQuerySchema: queryProps({ AspNetUserId: { type: "string", format: "uuid" } }),
    responseSchema: { type: "array", items: { type: "object" } },
  }),
  ep("Admin", "UpdateUserNotification", "POST", "jwt", "Update user notification", {
    requestBodySchema: queryProps({ IsRead: { type: "boolean" }, AspNetUserId: { type: "string", format: "uuid" }, PrePostId: { type: "integer" } }),
    responseSchema: { type: "integer" },
  }),
  ep("Admin", "PostNotificationsAsync", "GET", "jwt", "Post notifications", {
    requestQuerySchema: queryProps({ messageType: { type: "string" } }),
    responseSchema: arrayOf("NotificationResult"),
  }),
  ep("Admin", "CheckIfPublicHoliday", "GET", "jwt", "Public holiday check", {
    requestQuerySchema: queryProps({ dateTime: { type: "string", format: "date-time" } }),
    responseSchema: { type: "boolean" },
  }),
  ep("Admin", "SetIsMainUserProfileAsync", "GET", "jwt", "Set main user profile", {
    responseSchema: { type: "string", format: "uuid" },
  }),
];

export function wealthPath(ep: WealthEndpointDef): string {
  return `${ep.controller}/${ep.action}`;
}
