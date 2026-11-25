import { getDb } from "../db.js";
import { config } from "../config.js";
import { getAuth } from "./auth.js";

/**
 * Ensure a bootstrap admin exists (username/password). Idempotent:
 * does not reset password if the user already exists.
 */
export async function ensureBootstrapAdmin(): Promise<void> {
  const auth = getAuth();
  const db = getDb();
  const username = config.adminUsername.trim().toLowerCase();
  const existing = await db.collection("user").findOne({
    $or: [{ username }, { email: config.adminEmail }],
  });

  if (existing) {
    const updates: Record<string, unknown> = {};
    if (existing.role !== "admin") updates.role = "admin";
    if (existing.status !== "active") updates.status = "active";
    if (!existing.username) updates.username = username;
    if (Object.keys(updates).length > 0) {
      await db.collection("user").updateOne(
        { _id: existing._id },
        { $set: updates },
      );
      console.log(`[auth] Ensured admin role on existing user "${username}"`);
    } else {
      console.log(`[auth] Bootstrap admin already present: "${username}"`);
    }
    return;
  }

  try {
    await auth.api.signUpEmail({
      body: {
        email: config.adminEmail,
        password: config.adminPassword,
        name: "Admin",
        username,
      },
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    // Race or already created
    if (!/already|exist/i.test(msg)) {
      console.error("[auth] Failed to create bootstrap admin:", msg);
      throw err;
    }
  }

  await db.collection("user").updateOne(
    { username },
    {
      $set: {
        role: "admin",
        status: "active",
        companyName: "",
      },
    },
  );
  console.log(
    `[auth] Bootstrap admin ready: username="${username}" (change password after first login)`,
  );
}
