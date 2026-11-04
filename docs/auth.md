# Drug-Ya - Authentication & Session

Drug-Ya opens on a HOSxP **`officer`** login gate. The session is memory-only
inside the Tauri process. No login token, audit row, or password is written to
HOSxP or to disk.

---

## 1. Why officer, not opduser

The target sites keep staff accounts in `officer`; `opduser` is a different
(and in some gateway configurations blacklisted) credential store. Drug-Ya uses
only `officer`, matching the sibling Addict Order / ER-Refill / Med Recon
projects.

Never select:

- `officer_login_password` (plaintext);
- `opduser` columns;
- any session/audit table.

---

## 2. Login flow

```
open app
  -> probe saved connection (encrypted, keychain)
  -> probe backend session (memory)
  -> if no session: login gate
       -> officer login name + password
       -> one row from officer by login name
       -> active = 'Y'
       -> COALESCE(officer_authentication_type_id, 1) = 1
       -> MD5(password) = officer_login_password_md5 (case-insensitive)
       -> sanitized SessionUser in memory
  -> review console
```

The backend reads:

```sql
SELECT officer_id, officer_login_name, officer_login_password_md5,
       officer_active,
       CAST(COALESCE(officer_authentication_type_id, 1) AS SIGNED) AS auth_type_id,
       officer_doctor_code, officer_pname, officer_fname, officer_lname
FROM officer
WHERE officer_login_name = ?
LIMIT 1;
```

**Verified on Sra Bos:** `officer_authentication_type_id` is NULL for all 104
accounts. The query coalesces NULL to `1` (local DB); the command does not
require a literal stored `1`. Changing this to a strict `= 1` check would lock
every user out.

The MD5 digest stays in the backend. Only this crosses IPC:

```json
{
  "officerId": 12,
  "loginName": "pharm01",
  "displayName": "ภก. ตัวอย่าง",
  "doctorCode": null,
  "role": "STAFF"
}
```

---

## 3. The MD5 assumption

The comparison is:

```text
md5_hex(password).eq_ignore_ascii_case(officer_login_password_md5)
```

On Sra Bos the MD5 column is present and 32 characters for every account; the
auth-type column is NULL and is defaulted to local DB by the query. Both facts
were verified against the live schema.

The stored value must be 32 hex characters; malformed or empty digests never
match. This assumption is **not source-confirmed for every HOSxP build**. Test
one real account before the pilot:

1. Pick a test officer account.
2. Compute MD5 of the known password.
3. Compare with `officer_login_password_md5`.
4. Sign in through Drug-Ya.
5. Record the result in the site verification log.

If a site uses a different hash, do not weaken the check; add a documented,
tested verifier for that site.

---

## 4. Throttling

- 5 consecutive failures trigger a lockout.
- Base lockout 30 s, doubling up to 4 times (30, 60, 120, 240, 480 s).
- A successful login resets the counter.
- The lockout is in memory; restarting the process resets it, but the DB
  account remains the real boundary.

---

## 5. Session lifecycle

- Created only after password verification.
- Stored in `RwLock<Option<SessionUser>>` in the Tauri process.
- Cleared on:
  - explicit logout;
  - application restart;
  - connection change (new site = new identity scope);
  - connection clear.
- Every patient, review, and settings command checks for a session at the Tauri
  command boundary.
- The frontend clears every patient-scoped signal on logout:
  search results, selected patient, review, findings, clinical context,
  acknowledgement set, note, pregnancy override.
- A session epoch guards against a slow in-flight login restoring a session
  after logout/connection change.

---

## 6. Roles

v1 assigns `role = "STAFF"` to every signed-in officer. HOSxP group permissions
are often empty and are not a reliable app role. There is no role-based
restriction on rule settings or sign-off yet. The sign-off form records the
pharmacist's name and licence, but the app does not verify the licence against
an external registry.

Future work:

- map a verified `officer_group` to `PHARMACIST`/`ADMIN`;
- restrict rule changes and sign-off to a designated pharmacist role;
- add an idle lock that clears the review state;
- add a local audit trail only after a privacy/retention decision.

---

## 7. What is never logged

- HN, CID, patient names, addresses, phone numbers;
- drug lines or order details that could identify a patient;
- usernames/passwords;
- connection host/database/user/password.

Allowed:

- counts (search results, orders, findings);
- timings;
- error classes;
- redacted HN in a debug log (not in v1; avoid it).

---

## 8. Connection setup

- The login card offers **ตั้งค่า** when no connection is saved.
- The settings dialog has a connection section: host, port, database, user,
  password, test, save.
- Save connects first; a bad password is never persisted.
- Credentials are encrypted with `encryptman`; the master key lives in the OS
  keychain (`DrugYa` service).
- A connection change clears the session and the cached pool.
- The password is a `secrecy::SecretString` end to end in Rust; it is never
  returned over IPC.

---

## 9. Officer is not the prescriber

Drug-Ya does not print prescriptions and does not sign for anyone. The
prescriber shown on the review is from `opitemrece.doctor` or `ovst.doctor`;
the signed-in pharmacist is the reviewer. Do not conflate the two.
