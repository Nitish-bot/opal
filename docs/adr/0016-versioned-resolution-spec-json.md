# Publish Resolution Specs as versioned, schema-validated JSON

Resolution Specs use UTF-8 JSON that validates against a published, versioned JSON Schema. Devnet uses `schemaVersion: "1"` and the source schema at [resolution-spec-v1.schema.json](../schemas/resolution-spec-v1.schema.json). The required v1 fields are:

- `schemaVersion`: exactly `"1"`;
- `definitions`: an array of explicit term/meaning pairs, which may be empty;
- `sources`: a non-empty array ordered from highest to lowest authority, with each source declaring its name, URI, and `next-source` or `unresolvable` behavior when unavailable;
- `conflictBehavior`: exactly `highest-priority-wins` for v1;
- `ambiguityHandling`: non-empty normative statement-specific guidance that cannot override source ordering or protocol evidence timing;
- optional `rationale`: human-readable, non-normative context that cannot override any typed rule.

The schema rejects unknown fields. The final source cannot use `next-source`; creating clients enforce that cross-field rule in addition to JSON Schema validation. The onchain program stores only `resolution_spec_uri` and cannot parse or validate the uploaded JSON.

Devnet caps the exact uploaded UTF-8 JSON at 16 KiB (16,384 bytes). This is an Opal application-level resource bound, not an Arweave protocol restriction, upload tier, or pricing threshold. Implementations enforce it on the raw bytes before parsing and upload, then again on the retrieved bytes before signing Assertion creation. The limit deliberately remains independent of the selected Arweave uploader or gateway.

**Why.** Source ordering, unavailability behavior, and ambiguity rules cannot be reliably validated when buried in free-form Markdown. Versioned JSON gives creating clients, integrators, and the resolver one explicit contract, while documentation and applications can render the same object into participant-friendly prose. Rejecting unknown fields prevents an asserter from adding hidden normative instructions that compete with the agreed protocol rules.

**Consequences.** Creating clients fail closed before upload and again before signing Assertion creation if the exact content exceeds 16,384 bytes, is not valid UTF-8 JSON, does not validate against the declared schema, or fails a documented cross-field check. Existing immutable specs always retain their original version and meaning; a future schema version cannot reinterpret them. Public docs publish the JSON Schema, a rendered field reference, and complete valid and invalid examples. Whitespace and object-key order do not change semantics, but different uploaded bytes receive different Arweave IDs; no canonical-JSON serialization requirement is added for Devnet.
