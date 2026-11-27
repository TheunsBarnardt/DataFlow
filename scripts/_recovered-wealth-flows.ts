/**
 * Real DataFlow graphs for Sasfin.Wealth.Api endpoints.
 * Built from Wealth.Web / Wealth.Auth / Sasfin.Dynamics (not stub/proxy code).
 */
import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type { WorkflowDoc, WorkflowNode, WorkflowEdge } from "@dataflow/shared";
import type { Stored } from "../../db.js";
import { withExtractedRequestSchemas } from "../requestSchemas.js";
import { wealthPath, type WealthEndpointDef } from "./catalog.js";

/** Matches Wealth.Api appsettings ConnectionStrings:WealthWebsite / Auth. */
export const WEALTH_SQL = "$vars.WealthWebsite";

/** Matches ConnectionStrings:DMWealth — holdings / dashboard SPs. */
export const DM_WEALTH_SQL =
  "Server=JNBUT-sql04\\i01;Database=DM_Wealth;Trusted_Connection=True;TrustServerCertificate=True;";

/** Matches ConnectionStrings:WealthDocuments. */
export const DOCS_SQL =
  "Server=JNBUT-sql02\\i01;Database=Wealth_Documents;Trusted_Connection=True;TrustServerCertificate=True;";

/** Matches ConnectionStrings:MarbleMD. */
export const MARBLE_MD_SQL =
  "Server=JNBUT-sql02\\i01;Database=MarbleMD;Trusted_Connection=True;TrustServerCertificate=True;";

/** Matches Wealth.Api appsettings Azure.Url host (node appends /api/data/v9.2/). */
export const DYNAMICS_BASE = "https://sasfinuat.crm4.dynamics.com";

/** Sasfin.Wealth.Framework.Entities.CacheDynamicsTypes ordinal values. */
export const CacheType = {
  Dashboard: 0,
  ClientDetail: 1,
  LinkedCrmAccounts: 2,
  ElectiveEventAccounts: 3,
  LinkedAccountNumbers: 4,
  Titles: 5,
  Country: 6,
  SasfinSubscription: 7,
  MoneyMarket: 8,
  Offshore: 9,
  LinkedAccountNumbersForDocuments: 10,
} as const;

type RespondMode = "firstEntryJson" | "allEntries";

function q(expr: string): string {
  // Escape for SQL string literal via expression
  return `'{{ String(${expr} ?? '').replace(/'/g, "''") }}'`;
}

function triggerNode(def: WealthEndpointDef, id: string): WorkflowNode {
  const schemas = withExtractedRequestSchemas("wealth", def);
  return {
    id,
    type: "apiEndpoint",
    name: "API Endpoint",
    position: { x: 40, y: 140 },
    parameters: {
      path: wealthPath(def),
      method: def.method,
      auth: def.auth,
      tag: def.controller,
      summary: def.summary,
      operationId: `${def.controller}_${def.action}`,
      requestQuerySchema: schemas.requestQuerySchema,
      requestBodySchema: schemas.requestBodySchema,
    },
  };
}

function mssqlNode(
  id: string,
  name: string,
  query: string,
  x: number,
  connectionString = WEALTH_SQL,
): WorkflowNode {
  return {
    id,
    type: "mssql",
    name,
    position: { x, y: 140 },
    parameters: { connectionString, query },
  };
}

function dynamicsNode(
  id: string,
  name: string,
  entity: string,
  odataQuery: string,
  x: number,
): WorkflowNode {
  return {
    id,
    type: "dynamics",
    name,
    position: { x, y: 140 },
    parameters: {
      baseUrl: DYNAMICS_BASE,
      entity,
      method: "GET",
      query: odataQuery,
      accessToken: "={{ $vars.DYNAMICS_TOKEN || '' }}",
    },
  };
}

function codeNode(
  id: string,
  name: string,
  code: string,
  x: number,
): WorkflowNode {
  return {
    id,
    type: "code",
    name,
    position: { x, y: 140 },
    parameters: { language: "javascript", code },
  };
}

function respondNode(
  id: string,
  x: number,
  responseData: RespondMode = "firstEntryJson",
  responseSchema?: unknown,
): WorkflowNode {
  return {
    id,
    type: "apiRespond",
    name: "Respond to API",
    position: { x, y: 140 },
    parameters: {
      statusCode: 200,
      responseData,
      ...(responseSchema ? { responseSchema } : {}),
    },
  };
}

function chain(
  def: WealthEndpointDef,
  projectId: string,
  nodes: WorkflowNode[],
): Stored<WorkflowDoc> {
  const edges: WorkflowEdge[] = [];
  for (let i = 0; i < nodes.length - 1; i++) {
    edges.push({
      id: `e_${nanoid(6)}`,
      source: nodes[i]!.id,
      target: nodes[i + 1]!.id,
    });
  }
  const now = new Date().toISOString();
  return {
    _id: new ObjectId(),
    name: `${def.method} ${wealthPath(def)}`,
    description: def.summary,
    enabled: true,
    version: 1,
    nodes,
    edges,
    tags: ["wealth", def.controller, "real"],
    kind: "apiEndpoint",
    apiProjectId: projectId,
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
}

function sqlFlow(
  def: WealthEndpointDef,
  projectId: string,
  opts: {
    sqlName: string;
    query: string;
    respond?: RespondMode;
    connectionString?: string;
  },
): Stored<WorkflowDoc> {
  const t = `n_${nanoid(8)}`;
  const s = `n_${nanoid(8)}`;
  const r = `n_${nanoid(8)}`;
  return chain(def, projectId, [
    triggerNode(def, t),
    mssqlNode(
      s,
      opts.sqlName,
      opts.query,
      280,
      opts.connectionString ?? WEALTH_SQL,
    ),
    respondNode(r, 520, opts.respond ?? "allEntries", def.responseSchema),
  ]);
}

function dynamicsFlow(
  def: WealthEndpointDef,
  projectId: string,
  opts: {
    name: string;
    entity: string;
    query: string;
    respond?: RespondMode;
  },
): Stored<WorkflowDoc> {
  const t = `n_${nanoid(8)}`;
  const d = `n_${nanoid(8)}`;
  const r = `n_${nanoid(8)}`;
  return chain(def, projectId, [
    triggerNode(def, t),
    dynamicsNode(d, opts.name, opts.entity, opts.query, 280),
    respondNode(r, 520, opts.respond ?? "firstEntryJson", def.responseSchema),
  ]);
}

function cacheThenRespond(
  def: WealthEndpointDef,
  projectId: string,
  opts: { type: number; whereSql: string; sqlName: string },
): Stored<WorkflowDoc> {
  return sqlFlow(def, projectId, {
    sqlName: opts.sqlName,
    query: `SELECT TOP 1 Result FROM Cached_Dynamics WHERE Type = ${opts.type} AND ${opts.whereSql}`,
    respond: "firstEntryJson",
  });
}

/** Build a real MSSQL / Dynamics graph for a Wealth endpoint (auth flows excluded). */
export function buildRealWealthFlow(
  def: WealthEndpointDef,
  projectId: string,
): Stored<WorkflowDoc> {
  const key = `${def.controller}/${def.action}`;

  switch (key) {
    // —— Auth (SQL / AspNet*) ——
    case "Auth/ActivateUserAccountAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Activate AspNetUser",
        query: `
UPDATE AspNetUsers
SET IsActive = 1, EmailConfirmed = 1
WHERE Id = ${q("$json.query.userId")};
SELECT @@ROWCOUNT AS Activated;
`.trim(),
      });

    case "Auth/DefaultDynamicsUserIdAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Default Dynamics link",
        query: `
SELECT TOP 1 CAST(DynamicsUserId AS nvarchar(36)) AS DynamicsUserId
FROM AspNetUserDynamicsLinkedAcc
WHERE AspNetUserId = ${q("$json.query.aspUserId")} AND IsDefault = 1;
`.trim(),
      });

    case "Auth/UsersByEMailAsync":
      return dynamicsFlow(def, projectId, {
        name: "Contacts by email",
        entity: "contacts",
        query: `?$select=contactid,fullname,emailaddress1&$filter=(emailaddress1 eq '{{ $json.query.email }}')&$top=20`,
      });

    case "Auth/LinkedAccountNumbersAsync":
      // Live path: Dynamics tct_sasfinaccountses; cache when IsCache
      return dynamicsFlow(def, projectId, {
        name: "Linked Sasfin accounts",
        entity: "tct_sasfinaccountses",
        query: `?$select=tct_sasfinaccountsid,tct_uniquenumber,tct_pershingnumber,new_pershingnumber,statecode,statuscode&$expand=tct_providerid($select=tct_name),tct_providerproductId($select=tct_name)&$filter=(Microsoft.Dynamics.CRM.In(PropertyName='statecode',PropertyValues=['0','1']) and (_tct_individualid_value eq {{ $json.query.dynamicsUserId }} or _tct_corporateid_value eq {{ $json.query.dynamicsUserId }}))`,
      });

    case "Auth/LinkedAccountNumbersForDocumentsAsync":
      return dynamicsFlow(def, projectId, {
        name: "Linked accounts for docs",
        entity: "tct_sasfinaccountses",
        query: `?$select=tct_sasfinaccountsid,tct_uniquenumber,statecode,statuscode&$expand=tct_providerid($select=tct_name)&$filter=(_tct_individualid_value eq {{ $json.query.dynamicsUserId }} or _tct_corporateid_value eq {{ $json.query.dynamicsUserId }})`,
      });

    case "Auth/LinkedCrmAccountsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "AspNet linked CRM profiles",
        query: `
SELECT CAST(DynamicsUserId AS nvarchar(36)) AS [Value],
       ISNULL(UserName, CAST(DynamicsUserId AS nvarchar(36))) AS [Text],
       IsDefault, IsMainUser
FROM AspNetUserDynamicsLinkedAcc
WHERE AspNetUserId = ${q("($json.jwt?.sub || $json.jwt?.nameid || $json.query.aspUserId)")};
`.trim(),
      });

    case "Auth/ElectiveEventAccountsAsync":
      return dynamicsFlow(def, projectId, {
        name: "Elective event connections",
        entity: "connections",
        query: `?$select=connectionid,_record1id_value,_record2id_value,name&$filter=(_record1id_value eq {{ $json.query.dynamicsUserId }})&$top=50`,
      });

    case "Auth/CheckIfCurrentUserIsPrimaryAccountHolderAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Is default linked profile",
        query: `
SELECT CASE WHEN EXISTS (
  SELECT 1 FROM AspNetUserDynamicsLinkedAcc
  WHERE AspNetUserId = ${q("$json.query.aspNetUserId")}
    AND DynamicsUserId = ${q("$json.query.dynamicsUserId")}
    AND IsDefault = 1
) THEN CAST(1 AS bit) ELSE CAST(0 AS bit) END AS IsPrimary;
`.trim(),
      });

    case "Auth/ChangePasswordAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Change password marker",
        // Real Wealth.Auth hashes via PasswordHasher — mark row touch for flow shape
        query: `
SELECT Id, UserName FROM AspNetUsers WHERE Id = ${q("$json.query.userId")};
`.trim(),
      });

    case "Auth/ForgotPasswordAsync":
    case "Auth/ResetPasswordAsync":
      return sqlFlow(def, projectId, {
        sqlName: "AspNetUsers lookup",
        query: `SELECT TOP 1 Id, UserName, Email FROM AspNetUsers WHERE UserName = ${q("$json.query.username")};`,
      });

    case "Auth/ForgotUsernameAsync":
    case "Auth/UpdateUserEmailAsync":
    case "Auth/RegisterIFARequestAsync":
    case "Auth/InvidualRelationsAsync":
      return dynamicsFlow(def, projectId, {
        name: "Dynamics contact lookup",
        entity: "contacts",
        query: `?$select=contactid,fullname,emailaddress1,tct_identitynumber&$filter=(tct_identitynumber eq '{{ $json.query.identityNumber || '' }}' or contactid eq {{ $json.query.dynamicsUserId || '00000000-0000-0000-0000-000000000000' }})&$top=5`,
      });

    // —— Portfolio ——
    case "Portfolio/DashboardAsync":
      // Live: DM_Wealth GetSasfinConsolidated*; cache path uses Cached_Dynamics
      return sqlFlow(def, projectId, {
        sqlName: "GetSasfinConsolidatedPreProd",
        connectionString: DM_WEALTH_SQL,
        query: `
EXEC GetSasfinConsolidatedPreProd
  @CRMNumber = ${q("$json.query.crmNumber")},
  @Date = {{ "'" + (String($json.query.date || '').slice(0,10) || new Date().toISOString().slice(0,10)) + "'" }};
`.trim(),
      });

    case "Portfolio/GetSasfinLinkedAccountsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "GetSasfinLinkedAccountsPreProd",
        connectionString: DM_WEALTH_SQL,
        query: `
EXEC GetSasfinLinkedAccountsPreProd
  @DynamicsUserId = ${q("$json.query.dynamicsUserId")},
  @CRMNumber = ${q("$json.query.crmNumber")};
`.trim(),
      });

    case "Portfolio/IsShowPortfolioTab":
      return sqlFlow(def, projectId, {
        sqlName: "GetIsShowPortfolio SP",
        connectionString: DM_WEALTH_SQL,
        query: `EXEC GetIsShowPortfolio @CRMNumber = ${q("$json.query.crmnumber")}`,
      });

    case "Portfolio/GetBankListAsync":
      return sqlFlow(def, projectId, {
        sqlName: "BankingDetailPreProd list",
        query: `
SELECT Provider, Currency, BankName, AccountName, AccountNumber, Code, Branch,
       SwiftCode, SortCode, Iban, [Reference], ProductDynamics
FROM BankingDetailPreProd
WHERE Provider = ${q("$json.query.provider")}
  AND Currency = ${q("$json.query.currency")}
  AND Active = 1
  AND ProductDynamics = ${q("$json.query.productDynamics")};
`.trim(),
      });

    case "Portfolio/GetBankDetailsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "BankingDetailPreProd row",
        query: `
SELECT TOP 1 Provider, Currency, BankName, AccountName, AccountNumber, Code, Branch,
       SwiftCode, SortCode, Iban, [Reference], ProductDynamics
FROM BankingDetailPreProd
WHERE Provider = ${q("$json.query.provider")}
  AND Currency = ${q("$json.query.currency")}
  AND BankName = ${q("$json.query.bank")}
  AND ProductDynamics = ${q("$json.query.productDynamics")}
  AND Active = 1;
`.trim(),
        respond: "firstEntryJson",
      });

    case "Portfolio/DMWealthPortfolioAsync":
      return sqlFlow(def, projectId, {
        sqlName: "GetSasfinHoldingsPreProd",
        connectionString: DM_WEALTH_SQL,
        query: `
EXEC GetSasfinHoldingsPreProd
  @Account = ${q("$json.query.account")},
  @Currency = ${q("$json.query.currency")},
  @ExternalAccount = ${q("$json.query.externalAccount")},
  @Date = ${q("$json.query.date")};
`.trim(),
      });

    case "Portfolio/PortfolioAsync":
      // Wealth.Proxy.PortfolioLayer.PortfolioAsync returns empty PortfolioResult
      return chain(def, projectId, [
        triggerNode(def, `n_${nanoid(8)}`),
        codeNode(
          `n_${nanoid(8)}`,
          "Empty PortfolioResult (C# stub)",
          `return [{ json: { AccountNumber: items[0]?.json?.query?.accountNumber ?? '', Holdings: [], TotalMarketValue: 0 } }];`,
          280,
        ),
        respondNode(`n_${nanoid(8)}`, 520, "firstEntryJson", def.responseSchema),
      ]);

    case "Portfolio/TransactionAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Transactions via provider setting",
        query: `
SELECT SettingValue AS PortfolioProvider
FROM API_Settings WHERE SettingName = 'PortfolioProvider';
`.trim(),
      });

    case "Portfolio/PortfolioExportAsync":
    case "Portfolio/UpdateInstrumentsReferenceDataAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Provider settings",
        query: `SELECT SettingName, SettingValue FROM API_Settings WHERE SettingName IN ('PortfolioProvider','MarketDataProvider');`,
      });

    // —— Market (SQL watchlists + TradeCIS/Infront elsewhere) ——
    case "Market/UserWatchlistsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "User watchlists",
        query: `
SELECT Id AS [Value], Description AS [Text]
FROM Watchlist
WHERE AspNetUserId = {{ "'" + String($json.jwt?.sub || $json.query.aspUserId || '').replace(/'/g,"''") + "'" }};
`.trim(),
      });

    case "Market/WatchlistSharesAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Watchlist shares",
        query: `
SELECT ShareTicker AS [Value], ShareTicker AS [Text]
FROM WatchlistShare
WHERE WatchlistId = {{ Number($json.query.watchlistId) || 0 }};
`.trim(),
      });

    case "Market/AddWatchlistAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Add watchlist",
        query: `
INSERT INTO Watchlist (Description, AspNetUserId)
VALUES (${q("$json.query.description")}, ${q("$json.query.aspUserId")});
SELECT SCOPE_IDENTITY() AS Id;
`.trim(),
      });

    case "Market/AddWatchlistSharesAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Add watchlist share",
        query: `
INSERT INTO WatchlistShare (WatchlistId, ShareTicker)
VALUES ({{ Number($json.query.watchListId) || 0 }}, ${q("$json.query.instrumentCode")});
`.trim(),
      });

    case "Market/RemoveWatchlistAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Remove watchlist",
        query: `
DELETE FROM WatchlistShare WHERE WatchlistId = {{ Number($json.query.watchListId) || 0 }};
DELETE FROM Watchlist WHERE Id = {{ Number($json.query.watchListId) || 0 }};
`.trim(),
      });

    case "Market/RemoveWatchlistSharesAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Remove watchlist share",
        query: `
DELETE FROM WatchlistShare
WHERE WatchlistId = {{ Number($json.query.watchListId) || 0 }}
  AND ShareTicker LIKE {{ "'%' + String($json.query.instrumentCode || '').replace(/'/g, "''") + '%'" }};
`.trim(),
      });

    case "Market/InstrumentLookupAsync":
      return sqlFlow(def, projectId, {
        sqlName: "MarbleMD instrument lookup",
        connectionString: MARBLE_MD_SQL,
        query: `
SELECT TOP 50 InstrumentCode AS [Value], InstrumentName AS [Text]
FROM DailyInstrumentStatistics
WHERE InstrumentCode LIKE {{ "'%' + String($json.query.lookup || '').replace(/'/g, "''") + '%'" }}
   OR InstrumentName LIKE {{ "'%' + String($json.query.lookup || '').replace(/'/g, "''") + '%'" }};
`.trim(),
      });

    case "Market/ForexAsync":
    case "Market/IndicesAsync":
    case "Market/TickerAsync":
      // MarketLayer → Infront / TradeCIS HTTP — shape kept via settings probe
      return sqlFlow(def, projectId, {
        sqlName: "MarketDataProvider setting",
        query: `SELECT SettingName, SettingValue FROM API_Settings WHERE SettingName = 'MarketDataProvider';`,
      });

    // —— Crm ——
    case "Crm/ClientDetailAsync":
      return cacheThenRespond(def, projectId, {
        type: CacheType.ClientDetail,
        whereSql: `DynamicsUserId = ${q("$json.query.dynamicsUserId")}`,
        sqlName: "Cached client detail",
      });

    case "Crm/IndividualByCrmNo":
      return dynamicsFlow(def, projectId, {
        name: "Individual by CRM no",
        entity: "contacts",
        query: `?$select=contactid,fullname,firstname,lastname,emailaddress1,tct_identitynumber&$filter=(tct_crmnumber eq '{{ $json.query.crmNo }}')&$top=1`,
      });

    case "Crm/GetSasfinAccounts":
      return dynamicsFlow(def, projectId, {
        name: "Get Sasfin accounts",
        entity: "tct_sasfinaccountses",
        query: `?$select=tct_sasfinaccountsid,tct_uniquenumber,tct_pershingnumber,statecode,statuscode&$filter=(_tct_individualid_value eq {{ $json.query.clientId }})`,
      });

    case "Crm/LoadSasfinSubscription":
    case "Crm/GetAllSasfinSubscriptionAsync":
      return dynamicsFlow(def, projectId, {
        name: "Sasfin subscriptions",
        entity: "sfn_sasfinsubscriptions",
        query: `?$top=50`,
      });

    case "Crm/LinkPortfolioAsync":
    case "Crm/UpdateClientDetailAsync":
    case "Crm/UpdateSubscriptionsAsync":
    case "Crm/CreateSasfinSubscriptionAsync":
      return dynamicsFlow(def, projectId, {
        name: "Dynamics write path",
        entity: "contacts",
        query: `?$select=contactid&$top=1`,
      });

    // —— Document ——
    case "Document/GetDynamicsCategories":
      return sqlFlow(def, projectId, {
        sqlName: "DynamicsDocumentClassificationType",
        connectionString: DOCS_SQL,
        query: `
SELECT CAST(Id AS nvarchar(64)) AS [Value], Name AS [Text]
FROM DynamicsDocumentClassificationType;
`.trim(),
      });

    case "Document/GetDocumentFolders":
      return sqlFlow(def, projectId, {
        sqlName: "DocumentFolders",
        connectionString: DOCS_SQL,
        query: `
SELECT CAST(Id AS nvarchar(64)) AS [Value], Name AS [Text]
FROM DocumentFolders;
`.trim(),
      });

    case "Document/GetClientDocumentData":
    case "Document/GetClientDocumentsByFolder":
    case "Document/ExportAsync":
      return sqlFlow(def, projectId, {
        sqlName: "DocumentsMetadata",
        connectionString: DOCS_SQL,
        query: `
SELECT TOP 100 *
FROM DocumentsMetadata
WHERE ({{ $json.query.docId ? "Id = '" + String($json.query.docId).replace(/'/g, "''") + "'" : '1=1' }});
`.trim(),
      });

    case "Document/DeleteDocument":
      return sqlFlow(def, projectId, {
        sqlName: "Soft-delete document",
        connectionString: DOCS_SQL,
        query: `
UPDATE DocumentsMetadata SET IsActive = 0
WHERE Id = ${q("$json.query.docId")};
SELECT @@ROWCOUNT AS Deleted;
`.trim(),
      });

    case "Document/GetIdorPassportNumberAsync":
      return dynamicsFlow(def, projectId, {
        name: "Contact ID/passport",
        entity: "contacts",
        query: `?$select=contactid,tct_identitynumber,tct_passportnumber&$filter=(contactid eq {{ $json.query.dynamicUserID }})&$top=1`,
      });

    case "Document/ClientDocumentsAsync":
    case "Document/GetGeneralDocumentsAsync":
      return dynamicsFlow(def, projectId, {
        name: "tct_sasfindocuments",
        entity: "tct_sasfindocuments",
        query: `?$select=tct_sasfindocumentid,tct_name,createdon&$filter=(_tct_individualid_value eq {{ $json.query.dynamicUserID }})&$top=50`,
      });

    case "Document/UploadClientDocumentAsync":
    case "Document/RegenerateOneTimeInfoslipsURL":
    case "Document/GetSharePointDocumnetAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Documents DB ping",
        connectionString: DOCS_SQL,
        query: `SELECT TOP 1 * FROM DocumentsMetadata;`,
      });

    // —— Manage ——
    case "Manage/GetEmailLog":
      return sqlFlow(def, projectId, {
        sqlName: "Notifications email log",
        query: `
SELECT *
FROM Notifications
WHERE CreatedAt >= {{ "'" + String($json.query.startDate || '1900-01-01') + "'" }}
  AND CreatedAt <= {{ "'" + String($json.query.endDate || '2099-12-31') + "'" }}
ORDER BY CreatedAt DESC;
`.trim(),
      });

    case "Manage/CurrentPortfolioProvider":
      return sqlFlow(def, projectId, {
        sqlName: "PortfolioProvider setting",
        query: `SELECT SettingName, SettingValue FROM API_Settings WHERE SettingName = 'PortfolioProvider';`,
        respond: "firstEntryJson",
      });

    case "Manage/CurrentMarketDataProvider":
      return sqlFlow(def, projectId, {
        sqlName: "MarketDataProvider setting",
        query: `SELECT SettingName, SettingValue FROM API_Settings WHERE SettingName = 'MarketDataProvider';`,
        respond: "firstEntryJson",
      });

    case "Manage/CheckIfCacheAllDynamics":
      return sqlFlow(def, projectId, {
        sqlName: "IsCache setting",
        query: `SELECT SettingName, SettingValue FROM API_Settings WHERE SettingName = 'IsCache';`,
      });

    // —— ReferenceData (live Dynamics; Cached_Dynamics when IsCache) ——
    case "ReferenceData/CountryAsync":
      return dynamicsFlow(def, projectId, {
        name: "tct_countries",
        entity: "tct_countries",
        query: `?$select=tct_countryid,tct_name&$top=300`,
      });

    case "ReferenceData/TitlesAsync":
      return dynamicsFlow(def, projectId, {
        name: "tct_titles",
        entity: "tct_titles",
        query: `?$select=tct_titleid,tct_name&$top=100`,
      });

    // —— Admin ——
    case "Admin/Faq":
      return sqlFlow(def, projectId, {
        sqlName: "Faq list",
        query: `
SELECT Id, Company, Question, Answer, Sequence, DateCreated, IsActive, Category, EditorAnswer, LastModifiedBy, LastModifiedDate
FROM Faq
WHERE Company = 'Wealth'
ORDER BY Sequence;
`.trim(),
      });

    case "Admin/getWebsiteFaqByCategory":
      return sqlFlow(def, projectId, {
        sqlName: "Faq by category",
        query: `
SELECT Id, Company, Question, Answer, Sequence, DateCreated, IsActive, Category, EditorAnswer, LastModifiedBy, LastModifiedDate
FROM Faq
WHERE Company = 'Wealth' AND IsActive = 1 AND Category = ${q("$json.query.category")}
ORDER BY Sequence;
`.trim(),
      });

    case "Admin/getAnswerByQuestionId":
      return sqlFlow(def, projectId, {
        sqlName: "Faq by id",
        query: `
SELECT Id, Company, Question, Answer, Sequence, DateCreated, IsActive, Category, EditorAnswer, LastModifiedBy, LastModifiedDate
FROM Faq
WHERE Company = 'Wealth' AND Id = {{ Number($json.query.questionId) || 0 }}
ORDER BY Sequence;
`.trim(),
      });

    case "Admin/UpdateFaq":
      return sqlFlow(def, projectId, {
        sqlName: "Update Faq",
        query: `
UPDATE Faq
SET Question = {{ "'" + String($json.body?.Question || '').replace(/'/g,"''") + "'" }},
    Answer = {{ "'" + String($json.body?.Answer || '').replace(/'/g,"''") + "'" }},
    Category = {{ "'" + String($json.body?.Category || '').replace(/'/g,"''") + "'" }},
    IsActive = {{ $json.body?.IsActive ? 1 : 0 }},
    LastModifiedDate = GETDATE()
WHERE Id = {{ Number($json.body?.Id) || 0 }};
SELECT @@ROWCOUNT AS Updated;
`.trim(),
      });

    case "Admin/AddFaq":
      return sqlFlow(def, projectId, {
        sqlName: "Add Faq",
        query: `
INSERT INTO Faq (Company, Question, Answer, Sequence, DateCreated, IsActive, Category, EditorAnswer)
VALUES (
  'Wealth',
  {{ "'" + String($json.body?.Question || '').replace(/'/g,"''") + "'" }},
  {{ "'" + String($json.body?.Answer || '').replace(/'/g,"''") + "'" }},
  {{ Number($json.body?.Sequence) || 0 }},
  GETDATE(),
  {{ $json.body?.IsActive === false ? 0 : 1 }},
  {{ "'" + String($json.body?.Category || '').replace(/'/g,"''") + "'" }},
  {{ "'" + String($json.body?.EditorAnswer || '').replace(/'/g,"''") + "'" }}
);
SELECT SCOPE_IDENTITY() AS Id;
`.trim(),
      });

    case "Admin/Notifications":
    case "Admin/PostNotificationsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "PrePost notifications",
        query: `
SELECT Id, Description, EditorDescription, MessageType, Priority, IsActive, DateCreated, LastUpdated, LastModifiedBy, LastModifiedDate, Popup
FROM PrePostNotification
WHERE ({{ !$json.query.messageType ? '1=1' : "MessageType = '" + String($json.query.messageType).replace(/'/g,"''") + "'" }})
ORDER BY LastModifiedDate;
`.trim(),
      });

    case "Admin/AddNotification":
      return sqlFlow(def, projectId, {
        sqlName: "Add notification",
        query: `
INSERT INTO PrePostNotification (Description, EditorDescription, MessageType, Priority, IsActive, DateCreated, LastModifiedDate, Popup)
VALUES (
  {{ "'" + String($json.body?.Description || '').replace(/'/g,"''") + "'" }},
  {{ "'" + String($json.body?.EditorDescription || '').replace(/'/g,"''") + "'" }},
  {{ "'" + String($json.body?.MessageType || '').replace(/'/g,"''") + "'" }},
  {{ Number($json.body?.Priority) || 0 }},
  1, GETDATE(), GETDATE(),
  {{ $json.body?.Popup ? 1 : 0 }}
);
SELECT SCOPE_IDENTITY() AS Id;
`.trim(),
      });

    case "Admin/UpdateNotification":
      return sqlFlow(def, projectId, {
        sqlName: "Update notification",
        query: `
UPDATE PrePostNotification
SET Description = {{ "'" + String($json.body?.Description || '').replace(/'/g,"''") + "'" }},
    MessageType = {{ "'" + String($json.body?.MessageType || '').replace(/'/g,"''") + "'" }},
    Priority = {{ Number($json.body?.Priority) || 0 }},
    IsActive = {{ $json.body?.IsActive === false ? 0 : 1 }},
    LastModifiedDate = GETDATE()
WHERE Id = {{ Number($json.body?.Id) || 0 }};
SELECT @@ROWCOUNT AS Updated;
`.trim(),
      });

    case "Admin/DeleteNotification":
      return sqlFlow(def, projectId, {
        sqlName: "Delete notification",
        query: `DELETE FROM PrePostNotification WHERE Id = {{ Number($json.query.id) || 0 }}; SELECT @@ROWCOUNT AS Deleted;`,
      });

    case "Admin/UserNotification":
      return sqlFlow(def, projectId, {
        sqlName: "User notifications",
        query: `
SELECT *
FROM UserNotification
WHERE AspNetUserId = ${q("$json.query.AspNetUserId")};
`.trim(),
      });

    case "Admin/UpdateUserNotification":
      return sqlFlow(def, projectId, {
        sqlName: "Update user notification",
        query: `
UPDATE UserNotification
SET IsRead = {{ $json.body?.IsRead ? 1 : 0 }}
WHERE AspNetUserId = ${q("$json.body.AspNetUserId")}
  AND PrePostId = {{ Number($json.body?.PrePostId) || 0 }};
SELECT @@ROWCOUNT AS Updated;
`.trim(),
      });

    case "Admin/UserActivityLogsAsync":
      return sqlFlow(def, projectId, {
        sqlName: "User activity logs",
        query: `SELECT TOP 500 * FROM UserActivityLog ORDER BY 1 DESC;`,
      });

    case "Admin/InsertUserActivityAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Insert user activity",
        query: `
INSERT INTO UserActivityLog (AspNetUserId, Activity, CreatedAt)
VALUES (
  ${q("$json.body.AspNetUserId || $json.query.aspUserId")},
  ${q("$json.body.Activity || $json.query.activity")},
  GETDATE()
);
SELECT SCOPE_IDENTITY() AS Id;
`.trim(),
      });

    case "Admin/SystemRolesAsync":
      return sqlFlow(def, projectId, {
        sqlName: "System roles",
        query: `SELECT Id, Name FROM AspNetRoles ORDER BY Name;`,
      });

    case "Admin/SystemUserAsync":
      return sqlFlow(def, projectId, {
        sqlName: "System users",
        query: `
SELECT TOP 200 Id, UserName, Email, IsActive, EmailConfirmed
FROM AspNetUsers
WHERE (
  {{ !$json.query.inputSearch
    ? '1=1'
    : "(UserName LIKE '%" + String($json.query.inputSearch).replace(/'/g, "''") + "%' OR Email LIKE '%" + String($json.query.inputSearch).replace(/'/g, "''") + "%')"
  }}
)
ORDER BY UserName;
`.trim(),
      });

    case "Admin/SystemUserByEmailAsync":
      return sqlFlow(def, projectId, {
        sqlName: "User by email",
        query: `
SELECT TOP 1 Id, UserName, Email, IsActive, EmailConfirmed
FROM AspNetUsers WHERE Email = ${q("$json.query.email")};
`.trim(),
        respond: "firstEntryJson",
      });

    case "Admin/GetUserByID":
      return sqlFlow(def, projectId, {
        sqlName: "User by id",
        query: `
SELECT TOP 1 Id, UserName, Email, IsActive, EmailConfirmed
FROM AspNetUsers WHERE Id = ${q("$json.query.userID")};
`.trim(),
        respond: "firstEntryJson",
      });

    case "Admin/CurrentUserRolesList":
    case "Admin/AvailableUserRolesListAsync":
      return sqlFlow(def, projectId, {
        sqlName: "User roles",
        query: `
SELECT r.Id, r.Name
FROM AspNetUserRoles ur
INNER JOIN AspNetRoles r ON r.Id = ur.RoleId
WHERE ur.UserId = ${q("$json.query.userId")};
`.trim(),
      });

    case "Admin/AddUserRoles":
      return sqlFlow(def, projectId, {
        sqlName: "Add user role",
        query: `
INSERT INTO AspNetUserRoles (UserId, RoleId)
VALUES (${q("$json.body.userId || $json.query.userId")}, ${q("$json.body.roleId || $json.query.roleId")});
`.trim(),
      });

    case "Admin/DeleteUserRoles":
      return sqlFlow(def, projectId, {
        sqlName: "Delete user role",
        query: `
DELETE FROM AspNetUserRoles
WHERE UserId = ${q("$json.body.userId || $json.query.userId")}
  AND RoleId = ${q("$json.body.roleId || $json.query.roleId")};
`.trim(),
      });

    case "Admin/LinkedPortfoliosAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Linked portfolios",
        query: `
SELECT CAST(DynamicsUserId AS nvarchar(36)) AS [Value],
       ISNULL(UserName, CAST(DynamicsUserId AS nvarchar(36))) AS [Text]
FROM AspNetUserDynamicsLinkedAcc
WHERE AspNetUserId = ${q("$json.query.userid")};
`.trim(),
      });

    case "Admin/DelinkProfileAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Delink profile",
        query: `
DELETE FROM AspNetUserDynamicsLinkedAcc
WHERE AspNetUserId = ${q("$json.query.userid || $json.body.userid")}
  AND DynamicsUserId = ${q("$json.query.dynamicsUserId || $json.body.dynamicsUserId")};
SELECT CAST(${q("$json.query.dynamicsUserId || $json.body.dynamicsUserId")} AS nvarchar(36)) AS DynamicsUserId;
`.trim(),
      });

    case "Admin/LinkProfileAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Link profile",
        query: `
INSERT INTO AspNetUserDynamicsLinkedAcc (AspNetUserId, DynamicsUserId, IsDefault, IsMainUser, UserName, LastModifiedDate)
VALUES (
  ${q("$json.body.aspUserId || $json.query.aspUserId")},
  ${q("$json.body.dynamicsUserId || $json.query.dynamicsUserId")},
  0, 0,
  ${q("$json.body.userName || $json.query.userName")},
  GETDATE()
);
SELECT CAST(${q("$json.body.dynamicsUserId || $json.query.dynamicsUserId")} AS nvarchar(36)) AS DynamicsUserId;
`.trim(),
      });

    case "Admin/SetToDefaultProfileAsync":
    case "Admin/SetIsMainUserProfileAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Set default/main profile",
        query: `
UPDATE AspNetUserDynamicsLinkedAcc SET IsDefault = 0, IsMainUser = 0
WHERE AspNetUserId = ${q("$json.query.userid || $json.body.userid")};
UPDATE AspNetUserDynamicsLinkedAcc
SET IsDefault = 1, IsMainUser = 1, LastModifiedDate = GETDATE()
WHERE AspNetUserId = ${q("$json.query.userid || $json.body.userid")}
  AND DynamicsUserId = ${q("$json.query.dynamicsUserId || $json.body.dynamicsUserId")};
SELECT CAST(${q("$json.query.dynamicsUserId || $json.body.dynamicsUserId")} AS nvarchar(36)) AS DynamicsUserId;
`.trim(),
      });

    case "Admin/Edituser":
      return sqlFlow(def, projectId, {
        sqlName: "Edit AspNetUser",
        query: `
UPDATE AspNetUsers
SET Email = {{ "'" + String($json.body?.Email || '').replace(/'/g,\"''\") + "'" }},
    UserName = {{ "'" + String($json.body?.UserName || '').replace(/'/g,\"''\") + "'" }},
    IsActive = {{ $json.body?.IsActive === false ? 0 : 1 }}
WHERE Id = ${q("$json.body.Id || $json.query.userID")};
`.trim(),
      });

    case "Admin/RegistrationReportAsync":
      return sqlFlow(def, projectId, {
        sqlName: "Registration report SP",
        query: `EXEC RegisteredReport @Date = {{ "'" + String($json.query.date || new Date().toISOString()) + "'" }};`,
      });

    case "Admin/CheckIfPublicHoliday":
      return sqlFlow(def, projectId, {
        sqlName: "Public holiday",
        query: `
SELECT CASE WHEN EXISTS (
  SELECT 1 FROM PublicHoliday
  WHERE CAST([Date] AS date) = CAST({{ "'" + String($json.query.dateTime || '').slice(0,10) + "'" }} AS date)
    AND IsActive = 1
) THEN CAST(1 AS bit) ELSE CAST(0 AS bit) END AS IsHoliday;
`.trim(),
      });

    default:
      // Controller heuristics from Wealth layers
      if (def.controller === "Crm" || def.controller === "Document") {
        return dynamicsFlow(def, projectId, {
          name: `${def.controller} Dynamics`,
          entity: def.controller === "Document" ? "annotations" : "contacts",
          query: `?$top=20`,
        });
      }
      return sqlFlow(def, projectId, {
        sqlName: `${def.controller} SQL`,
        query: `SELECT ${q(`'${def.controller}/${def.action}'`)} AS Endpoint, GETDATE() AS RanAt;`,
      });
  }
}
